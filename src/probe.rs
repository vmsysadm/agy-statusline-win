use std::collections::HashSet;
use std::fs::{self, File};
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use crate::data::InputData;

/// Markers a task log carries once the task is over.
///
/// Most background tasks announce completion in the transcript as a SYSTEM_MESSAGE with
/// `sender=<conv>/task-<id>`. Timers cancelled by an early-termination condition never do —
/// the task that satisfied the condition reports instead, and the timer dies silently. Those
/// only ever record their fate in `tasks/task-<id>.log`, so it is the authoritative source.
const TASK_LOG_DONE_MARKERS: [&str; 2] = ["Timer cancelled", "Timer fired"];

/// Phrase the harness emits when a tool is handed off to the background.
const TASK_START_MARKER: &str = "Tool is running as a background task with task id: ";

/// Phrase the harness emits when `manage_subagents` terminates subagents. This is the only
/// explicit end-of-life a subagent gets: they do not retire themselves, and nothing in the
/// parent transcript marks one as finished.
const SUBAGENT_KILL_MARKER: &str = "Successfully killed";

/// How long after its last transcript write a subagent still counts as working, once its
/// transcript tail shows it mid-tool-call rather than finished.
///
/// A subagent writes a step every second or two while it works (p95 gap 2-10s in real
/// sessions), but a single long tool call can leave its transcript untouched much longer --
/// 52s was the worst observed. The window has to clear that or the indicator drops out in
/// the middle of a build; the cost of clearing it is that a subagent which has gone quiet
/// keeps the indicator lit for up to this long.
const SUBAGENT_ACTIVE_WINDOW: std::time::Duration = std::time::Duration::from_secs(90);

/// Represents the real-time activity status probed from session transcripts.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub(crate) struct SessionActivity {
    pub(crate) is_blocked_on_question: bool,
    pub(crate) active_tasks: usize,
    pub(crate) active_subagents: usize,
}

/// Resolve the actual transcript file path on disk.
/// Antigravity CLI's payload provides `transcript_path` as:
/// `~/.gemini/antigravity/brain/<id>/...`
/// BUT the directory on disk for the CLI is:
/// `~/.gemini/antigravity-cli/brain/<id>/...`!
fn resolve_transcript_path(data: &InputData) -> Option<PathBuf> {
    if let Some(raw) = data.transcript_path.as_deref() {
        let p = Path::new(raw);
        if p.exists() {
            return Some(p.to_path_buf());
        }

        // Try replacing \antigravity\brain with \antigravity-cli\brain
        let adjusted_str = raw.replace(r"\.gemini\antigravity\brain\", r"\.gemini\antigravity-cli\brain\")
                              .replace("/.gemini/antigravity/brain/", "/.gemini/antigravity-cli/brain/");
        let adjusted = PathBuf::from(adjusted_str);
        if adjusted.exists() {
            return Some(adjusted);
        }
    }

    // Fallback: derive from conversation_id if available
    if let Some(conv_id) = data.conversation_id.as_deref() {
        if let Ok(home) = std::env::var("USERPROFILE").or_else(|_| std::env::var("HOME")) {
            let p = PathBuf::from(home)
                .join(".gemini")
                .join("antigravity-cli")
                .join("brain")
                .join(conv_id)
                .join(".system_generated")
                .join("logs")
                .join("transcript.jsonl");
            if p.exists() {
                return Some(p);
            }
        }
    }

    None
}

/// Conversation id that owns a transcript:
/// `<brain>/<conv>/.system_generated/logs/transcript.jsonl` -> `<conv>`.
fn transcript_conversation_id(transcript: &Path) -> Option<String> {
    Some(
        transcript
            .parent()?
            .parent()?
            .parent()?
            .file_name()?
            .to_string_lossy()
            .into_owned(),
    )
}

/// Split a raw task reference into `(owning conversation, task key)`.
///
/// Only `task-<digits>` — optionally prefixed with `<conv>/` — is a task id. Anything else is
/// prose that merely quotes the marker phrase (an assistant explaining the probe, a transcript
/// pasted into a tool result) and must never register as a running task.
fn split_task_ref(raw: &str) -> Option<(Option<&str>, &str)> {
    let (conv, key) = match raw.rsplit_once('/') {
        Some((conv, key)) => (Some(conv), key),
        None => (None, raw),
    };
    let digits = key.strip_prefix("task-")?;
    if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    Some((conv, key))
}

/// True for a canonical `8-4-4-4-12` hex UUID — the shape of a subagent conversation id.
/// Anything else quoting `conversationId` (source code, a pasted transcript) is not a subagent.
fn is_uuid(s: &str) -> bool {
    const GROUPS: [usize; 5] = [8, 4, 4, 4, 12];
    let mut parts = s.split('-');
    GROUPS
        .iter()
        .all(|n| parts.next().is_some_and(|p| p.len() == *n && p.bytes().all(|b| b.is_ascii_hexdigit())))
        && parts.next().is_none()
}

/// Take the token that follows a marker, stopping at the JSON/whitespace boundary.
fn token(after: &str) -> &str {
    let end = after
        .find(|c: char| c.is_whitespace() || c == '\\' || c == '"')
        .unwrap_or(after.len());
    after[..end].trim()
}

/// The brain root that holds every conversation, derived from one conversation's transcript:
/// `<brain>/<conv>/.system_generated/logs/transcript.jsonl` -> `<brain>`.
///
/// A subagent is a conversation in its own right, so its transcript is a sibling of the
/// parent's under this directory.
fn brain_dir(transcript: &Path) -> Option<PathBuf> {
    Some(transcript.parent()?.parent()?.parent()?.parent()?.to_path_buf())
}

/// The last non-empty line of a transcript, read from its tail.
fn last_step(path: &Path, len: u64) -> Option<String> {
    const TAIL: u64 = 16384;
    let mut file = File::open(path).ok()?;
    let read = len.min(TAIL);
    file.seek(SeekFrom::End(-(read as i64))).ok()?;
    let mut buf = vec![0u8; read as usize];
    file.read_exact(&mut buf).ok()?;
    String::from_utf8_lossy(&buf)
        .lines()
        .rev()
        .find(|l| !l.trim().is_empty())
        .map(str::to_string)
}

/// True when a transcript's last step is a model reply that ended the turn: a `PLANNER_RESPONSE`
/// carrying prose and no tool calls.
///
/// Every step a working agent writes is either a tool call or its result, so this shape only
/// appears when the agent has yielded. A subagent has no user of its own to prompt it, so a
/// finished turn is a finished subagent; it resumes only if the parent messages it, and that
/// appends a SYSTEM_MESSAGE which moves the tail off this shape again.
///
/// The line must be a complete JSON object: a tail caught mid-write can end after `status`
/// but before the `tool_calls` that would have disqualified it.
fn transcript_turn_ended(path: &Path, len: u64) -> bool {
    last_step(path, len).is_some_and(|line| {
        let line = line.trim_end();
        line.ends_with('}')
            && line.contains("\"type\":\"PLANNER_RESPONSE\"")
            && line.contains("\"status\":\"DONE\"")
            && !line.contains("\"tool_calls\"")
    })
}

/// True while a subagent still looks like it is doing work.
///
/// The parent transcript cannot answer this. A subagent sends the parent many messages over
/// its life -- progress reports, findings, questions -- all structurally identical, so there
/// is no "it finished" message to watch for; treating the first one as completion is what
/// used to blank the indicator seconds after a subagent started. What *does* track activity
/// is the subagent's own transcript, which grows as it works and stops when it does not.
///
/// A subagent whose transcript does not exist yet counts as working: the parent records the
/// spawn before the child writes its first step, and a just-created subagent is precisely
/// when the indicator should light up.
fn subagent_is_working(brain: &Path, id: &str) -> bool {
    let transcript = brain
        .join(id)
        .join(".system_generated")
        .join("logs")
        .join("transcript.jsonl");

    let meta = match fs::metadata(&transcript) {
        Ok(m) => m,
        Err(_) => return true,
    };

    // The subagent's own last step says so directly when it has finished; the idle window below
    // is only the fallback for a subagent sitting inside a long tool call.
    if transcript_turn_ended(&transcript, meta.len()) {
        return false;
    }

    let modified = match meta.modified() {
        Ok(m) => m,
        Err(_) => return true,
    };

    // A modification stamped in the future (clock skew, a file copied across machines) is not
    // evidence of idleness, so treat an unmeasurable age as recent.
    std::time::SystemTime::now()
        .duration_since(modified)
        .map(|age| age <= SUBAGENT_ACTIVE_WINDOW)
        .unwrap_or(true)
}

/// Locate the per-task log directory that sits alongside the transcript:
/// `<brain>/<conv>/.system_generated/logs/transcript.jsonl` -> `<...>/.system_generated/tasks`.
fn tasks_dir(transcript: &Path) -> Option<PathBuf> {
    Some(transcript.parent()?.parent()?.join("tasks"))
}

/// True if `tasks/<task_id>.log` shows the task already reached a terminal state.
/// A missing or unreadable log is not evidence of completion.
fn finished_per_task_log(tasks_dir: &Path, task_id: &str) -> bool {
    fs::read_to_string(tasks_dir.join(format!("{}.log", task_id)))
        .map(|log| TASK_LOG_DONE_MARKERS.iter().any(|m| log.contains(m)))
        .unwrap_or(false)
}

/// Fast probe to inspect session transcript for:
/// 1. If agent is blocked waiting for user input on `ask_question`
/// 2. Any active background tasks (launched and not yet finished)
/// 3. Any active subagents (invoked and not yet reported finished)
pub(crate) fn probe_session_activity(data: &InputData) -> SessionActivity {
    let mut activity = SessionActivity::default();

    let path = match resolve_transcript_path(data) {
        Some(p) => p,
        None => return activity,
    };

    let mut file = match File::open(&path) {
        Ok(f) => f,
        Err(_) => return activity,
    };

    let len = match file.metadata() {
        Ok(m) => m.len(),
        Err(_) => return activity,
    };

    if len == 0 {
        return activity;
    }

    // Read the tail of the transcript. The window has to outlive a whole turn's worth of tool
    // output: if a task's start line scrolls out while its completion is still inside, the task
    // silently stops being counted. 256KB covers all but the longest sessions and still parses
    // in well under a millisecond.
    let read_size = len.min(262144);
    if file.seek(SeekFrom::End(-(read_size as i64))).is_err() {
        return activity;
    }

    let mut buf = vec![0u8; read_size as usize];
    if file.read_exact(&mut buf).is_err() {
        return activity;
    }

    let mut text = String::from_utf8_lossy(&buf).into_owned();

    // A tail read almost always starts mid-line; that fragment is not parseable JSON and its
    // trailing half of a task id would be garbage, so drop it.
    if read_size < len {
        if let Some(nl) = text.find('\n') {
            text.drain(..=nl);
        }
    }

    // Task ids carry the conversation that owns them. A transcript quoted inside a tool result
    // (reading another session's log) carries a foreign prefix and must be ignored.
    let conv_id = transcript_conversation_id(&path);
    let owns = |conv: Option<&str>| match (conv, conv_id.as_deref()) {
        (Some(c), Some(mine)) => c == mine,
        _ => true,
    };

    // 1. Check if the latest action was ask_question without subsequent user answer
    activity.is_blocked_on_question = text
        .lines()
        .rev()
        .find(|l| !l.trim().is_empty())
        .is_some_and(|l| l.contains("\"ask_question\""));

    // 2. Track background tasks:
    // Started tasks match: `task id: .../task-<id>` or `"task id: \".../task-<id>\""`
    // Completed tasks match: `sender=.../task-<id>` and `finished with result:`
    let mut started_tasks = HashSet::new();
    let mut completed_tasks = HashSet::new();

    // 3. Track subagents:
    // Started: `"INVOKE_SUBAGENT"` with `"conversationId": "<uuid>"`
    // Ended:   only an explicit `manage_subagents` kill. A subagent's own messages say
    //          nothing about whether it is finished, so liveness is decided afterwards from
    //          each subagent's transcript -- see `subagent_is_working`.
    let mut started_subagents = HashSet::new();

    for line in text.lines() {
        // Look for started background tasks. Guard against file viewing / command echoes of the phrase.
        if !line.contains("File Path:")
            && !line.contains("Output:")
            && !line.contains("\\\"Tool is running as a background task with task id:")
        {
            for (pos, _) in line.match_indices(TASK_START_MARKER) {
                // Extract task ID (e.g., `<conv>/task-10` or `task-10`)
                if let Some((conv, key)) = split_task_ref(token(&line[pos + TASK_START_MARKER.len()..])) {
                    if owns(conv) {
                        started_tasks.insert(key.to_string());
                    }
                }
            }
        }

        // Look for completed background tasks or subagents. A single SYSTEM_MESSAGE step can
        // carry several `[Message]` blocks, so every sender on the line counts.
        // A single SYSTEM_MESSAGE step can carry several `[Message]` blocks, so every sender
        // on the line counts. Only task senders retire anything here: a bare-UUID sender is a
        // subagent reporting in, which it does repeatedly while still working.
        if line.contains("\"type\":\"SYSTEM_MESSAGE\"") {
            for (pos, _) in line.match_indices("sender=") {
                let sender = token(&line[pos + 7..]);
                if let Some((conv, key)) = split_task_ref(sender) {
                    if owns(conv) {
                        completed_tasks.insert(key.to_string());
                    }
                }
            }
        }

        // `manage_subagents` kills every live subagent at once and reports the count. Lines are
        // walked in order, so clearing here leaves any subagent spawned later still tracked.
        if line.contains(SUBAGENT_KILL_MARKER) && line.contains("subagent") && !line.contains("File Path:") {
            started_subagents.clear();
        }

        // Look for started subagents
        // Subagent start event output has: "Created the following subagents:\n{\n  \"conversationId\": \"...\""
        // Guard against file viewing / command echoes of the phrase
        if (line.contains("Created the following subagents") || line.contains("\"type\":\"INVOKE_SUBAGENT\""))
            && !line.contains("File Path:")
            && !line.contains("Output:")
        {
            for (pos, _) in line.match_indices("conversationId") {
                let after = &line[pos + 14..];
                let trimmed = after.trim_start_matches(|c: char| c == ':' || c == '\\' || c == '"' || c.is_whitespace());
                let end = trimmed.find(|c: char| c == '\\' || c == '"' || c.is_whitespace() || c == ',').unwrap_or(trimmed.len());
                let sub_id = &trimmed[..end];
                if is_uuid(sub_id) {
                    started_subagents.insert(sub_id.to_string());
                }
            }
        }
    }

    // A task with no completion message in the transcript may still be over — check its own
    // log before counting it as active. Only the handful of unmatched ids are read.
    let tasks_dir = tasks_dir(&path);
    activity.active_tasks = started_tasks
        .difference(&completed_tasks)
        .filter(|id| !tasks_dir.as_ref().is_some_and(|d| finished_per_task_log(d, id)))
        .count();

    // Every subagent still tracked was spawned and never killed; whether it is *working* is a
    // question only its own transcript can answer. There are only ever a handful, so this is
    // one stat call each.
    activity.active_subagents = match brain_dir(&path) {
        Some(brain) => started_subagents
            .iter()
            .filter(|id| subagent_is_working(&brain, id))
            .count(),
        None => started_subagents.len(),
    };

    activity
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use std::io::Write;

    /// A temp directory that cleans itself up.
    ///
    /// Cargo runs tests in parallel threads of one process, so a fixture path has to be unique per
    /// test *and* per concurrent `cargo test` run -- a fixed name under the shared system temp dir
    /// means two runs silently clobber each other's transcripts. Cleaning up on [`Drop`] rather
    /// than at the end of the test body also means a failing assertion no longer leaks the tree:
    /// a panic unwinds past a trailing `remove_dir_all`, but not past a destructor.
    pub(crate) struct ScratchDir(PathBuf);

    impl ScratchDir {
        pub(crate) fn path(&self) -> &Path {
            &self.0
        }
    }

    impl Drop for ScratchDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    pub(crate) fn scratch_dir(label: &str) -> ScratchDir {
        use std::sync::atomic::{AtomicUsize, Ordering};
        static SEQ: AtomicUsize = AtomicUsize::new(0);

        let root = std::env::temp_dir().join(format!(
            "agy-test-{}-{}-{}",
            label,
            std::process::id(),
            SEQ.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&root).unwrap();
        ScratchDir(root)
    }

    /// Build a brain-shaped session dir: `<scratch>/sess/.system_generated/{logs,tasks}`.
    /// The conversation dir must be named `sess` so the `sess/task-N` ids below are owned by it.
    ///
    /// The returned [`ScratchDir`] owns the tree: bind it for the whole test (`let (_root, ..)`),
    /// because dropping it early deletes the fixture out from under the probe.
    fn session(name: &str) -> (ScratchDir, PathBuf, PathBuf) {
        let root = scratch_dir(name);
        let logs = root.path().join("sess").join(".system_generated").join("logs");
        let tasks = root.path().join("sess").join(".system_generated").join("tasks");
        std::fs::create_dir_all(&logs).unwrap();
        std::fs::create_dir_all(&tasks).unwrap();
        (root, logs.join("transcript.jsonl"), tasks)
    }

    fn data_for(path: &Path) -> InputData {
        let mut data = InputData::default();
        data.transcript_path = Some(path.to_str().unwrap().to_string());
        data
    }

    #[test]
    fn test_probe_question_blocked() {
        let temp_dir = std::env::temp_dir();
        let test_path = temp_dir.join("test_transcript_probe_q.jsonl");

        {
            let mut f = File::create(&test_path).unwrap();
            writeln!(f, "{{\"step_index\":1,\"source\":\"USER_EXPLICIT\",\"type\":\"USER_INPUT\",\"content\":\"hi\"}}").unwrap();
            writeln!(f, "{{\"step_index\":2,\"source\":\"MODEL\",\"type\":\"PLANNER_RESPONSE\",\"tool_calls\":[{{\"name\":\"ask_question\",\"args\":{{}}}}]}}").unwrap();
        }

        let mut data = InputData::default();
        data.transcript_path = Some(test_path.to_str().unwrap().to_string());
        let act = probe_session_activity(&data);
        assert!(act.is_blocked_on_question);
        assert_eq!(act.active_tasks, 0);
        assert_eq!(act.active_subagents, 0);

        // User answers
        {
            let mut f = File::options().append(true).open(&test_path).unwrap();
            writeln!(f, "{{\"step_index\":3,\"source\":\"USER\",\"type\":\"GENERIC\",\"content\":\"Option 1\"}}").unwrap();
        }
        let act2 = probe_session_activity(&data);
        assert!(!act2.is_blocked_on_question);

        let _ = std::fs::remove_file(&test_path);
    }

    #[test]
    fn test_probe_active_background_tasks() {
        let (_root, test_path, _tasks) = session("agy_probe_tasks");

        {
            let mut f = File::create(&test_path).unwrap();
            writeln!(f, "{{\"step_index\":1,\"content\":\"Tool is running as a background task with task id: sess/task-10\\nTask Description: sleep 10\"}}").unwrap();
        }

        let data = data_for(&test_path);
        let act = probe_session_activity(&data);
        assert_eq!(act.active_tasks, 1);

        // Task completes
        {
            let mut f = File::options().append(true).open(&test_path).unwrap();
            writeln!(f, "{{\"step_index\":2,\"type\":\"SYSTEM_MESSAGE\",\"content\":\"[Message] sender=sess/task-10 priority=MESSAGE_PRIORITY_HIGH content=Task id finished with result: ok\"}}").unwrap();
        }
        // Another task starts via schedule tool
        {
            let mut f = File::options().append(true).open(&test_path).unwrap();
            writeln!(f, "{{\"step_index\":3,\"content\":\"Tool is running as a background task with task id: sess/task-11\\nTask Description: Timer: 10s\"}}").unwrap();
        }
        let act3 = probe_session_activity(&data);
        assert_eq!(act3.active_tasks, 1);

        // Timer completes with custom notification prompt
        {
            let mut f = File::options().append(true).open(&test_path).unwrap();
            writeln!(f, "{{\"step_index\":4,\"type\":\"SYSTEM_MESSAGE\",\"content\":\"[Message] sender=sess/task-11 priority=MESSAGE_PRIORITY_HIGH content=The 10-second timer completed.\"}}").unwrap();
        }
        let act4 = probe_session_activity(&data);
        assert_eq!(act4.active_tasks, 0);

    }

    /// Prose that merely quotes the start marker — the assistant explaining how this probe
    /// works — used to register `<id>` as a task that could never complete, pinning ⚙ on the
    /// tab title forever.
    #[test]
    fn test_probe_ignores_prose_quoting_the_marker() {
        let (_root, test_path, _tasks) = session("agy_probe_prose");

        {
            let mut f = File::create(&test_path).unwrap();
            writeln!(f, "{{\"step_index\":1,\"type\":\"GENERIC\",\"status\":\"RUNNING\",\"content\":\"Tool is running as a background task with task id: sess/task-72\\nTask Description: grep\"}}").unwrap();
            writeln!(f, "{{\"step_index\":2,\"type\":\"SYSTEM_MESSAGE\",\"content\":\"[Message] sender=sess/task-72 content=finished with result: ok\"}}").unwrap();
            writeln!(f, "{{\"step_index\":3,\"type\":\"PLANNER_RESPONSE\",\"content\":\"It scans each line for `Tool is running as a background task with task id: <id>` and inserts each ID into a set.\"}}").unwrap();
        }

        assert_eq!(probe_session_activity(&data_for(&test_path)).active_tasks, 0);

    }

    /// Another session's transcript, pasted into a tool result, carries a foreign conversation
    /// prefix — its tasks belong to that session, not this one.
    #[test]
    fn test_probe_ignores_foreign_conversation_tasks() {
        let (_root, test_path, _tasks) = session("agy_probe_foreign");

        {
            let mut f = File::create(&test_path).unwrap();
            writeln!(f, "{{\"step_index\":1,\"type\":\"GENERIC\",\"content\":\"Created At: now\\nTool is running as a background task with task id: other-conv/task-17\\nTask Description: cat\"}}").unwrap();
        }

        assert_eq!(probe_session_activity(&data_for(&test_path)).active_tasks, 0);

    }

    /// One SYSTEM_MESSAGE step can batch several `[Message]` blocks; every completion in it
    /// must be credited, not just the first.
    #[test]
    fn test_probe_multiple_completions_on_one_line() {
        let (_root, test_path, _tasks) = session("agy_probe_batched");

        {
            let mut f = File::create(&test_path).unwrap();
            writeln!(f, "{{\"step_index\":1,\"content\":\"Tool is running as a background task with task id: sess/task-1\"}}").unwrap();
            writeln!(f, "{{\"step_index\":2,\"content\":\"Tool is running as a background task with task id: sess/task-2\"}}").unwrap();
            writeln!(f, "{{\"step_index\":3,\"type\":\"SYSTEM_MESSAGE\",\"content\":\"[Message] sender=sess/task-1 content=done\\n[Message] sender=sess/task-2 content=done\"}}").unwrap();
        }

        assert_eq!(probe_session_activity(&data_for(&test_path)).active_tasks, 0);

    }

    /// A timer cancelled by an early-termination condition never writes a completion
    /// SYSTEM_MESSAGE — only its own task log records that it is over.
    #[test]
    fn test_probe_cancelled_timer_is_not_active() {
        let (_root, test_path, tasks) = session("agy_probe_cancelled_timer");

        // A shell task and a timer both start; only the shell task reports back.
        {
            let mut f = File::create(&test_path).unwrap();
            writeln!(f, "{{\"step_index\":1,\"status\":\"RUNNING\",\"content\":\"Tool is running as a background task with task id: sess/task-14\\nTask Description: cargo build\"}}").unwrap();
            writeln!(f, "{{\"step_index\":2,\"status\":\"RUNNING\",\"content\":\"Tool is running as a background task with task id: sess/task-18\\nTask Description: Timer: 30s\"}}").unwrap();
            writeln!(f, "{{\"step_index\":3,\"type\":\"SYSTEM_MESSAGE\",\"content\":\"[Message] sender=sess/task-14 content=done\"}}").unwrap();
        }

        let data = data_for(&test_path);

        // Timer still pending: no log yet, so it counts as active.
        assert_eq!(probe_session_activity(&data).active_tasks, 1);

        // Timer cancelled because task-14 reported first.
        std::fs::write(
            tasks.join("task-18.log"),
            "Successfully scheduled one-shot timer.\nEarly Termination: sess/task-14\n\n\
             [2026-09-04T13:37:26-06:00] Timer cancelled: early-termination condition \
             (sess/task-14) met by a message from sess/task-14.\n",
        )
        .unwrap();
        assert_eq!(probe_session_activity(&data).active_tasks, 0);

    }

    /// A task log with no terminal marker must not be mistaken for a completion.
    #[test]
    fn test_probe_running_task_with_log_stays_active() {
        let (_root, test_path, tasks) = session("agy_probe_running_task");

        {
            let mut f = File::create(&test_path).unwrap();
            writeln!(f, "{{\"step_index\":1,\"status\":\"RUNNING\",\"content\":\"Tool is running as a background task with task id: sess/task-9\\nTask Description: cargo build\"}}").unwrap();
        }
        std::fs::write(tasks.join("task-9.log"), "   Compiling agy-statusline v0.11.0\n").unwrap();

        assert_eq!(probe_session_activity(&data_for(&test_path)).active_tasks, 1);

    }

    const SUB_ID: &str = "6f1c2d34-88d5-4cb1-a308-20e52d1087e2";

    /// Write a subagent transcript as a sibling conversation under the same brain root,
    /// aged `secs_ago` seconds.
    fn subagent_transcript(parent_transcript: &Path, id: &str, secs_ago: u64) {
        let brain = brain_dir(parent_transcript).unwrap();
        let logs = brain.join(id).join(".system_generated").join("logs");
        std::fs::create_dir_all(&logs).unwrap();
        let path = logs.join("transcript.jsonl");
        std::fs::write(&path, "{\"step_index\":1,\"status\":\"DONE\"}\n").unwrap();
        let when = std::time::SystemTime::now() - std::time::Duration::from_secs(secs_ago);
        File::options().write(true).open(&path).unwrap().set_modified(when).unwrap();
    }

    fn spawn_line(step: u32, id: &str) -> String {
        format!(
            "{{\"step_index\":{},\"type\":\"GENERIC\",\"content\":\"Created the following subagents:\\n{{\\n  \\\"conversationId\\\":  \\\"{}\\\"\\n}}\"}}",
            step, id
        )
    }

    /// A subagent counts as working while its own transcript is still being written, and
    /// stops counting once it has gone quiet -- not when it first reports to the parent.
    #[test]
    fn test_probe_subagent_activity_follows_its_transcript() {
        let (_root, test_path, _tasks) = session("agy_probe_subagents");
        {
            let mut f = File::create(&test_path).unwrap();
            writeln!(f, "{}", spawn_line(1, SUB_ID)).unwrap();
        }
        let data = data_for(&test_path);

        // Freshly written transcript: working.
        subagent_transcript(&test_path, SUB_ID, 0);
        assert_eq!(probe_session_activity(&data).active_subagents, 1);

        // Long tool call -- quiet for a while, but still inside the window.
        subagent_transcript(&test_path, SUB_ID, 60);
        assert_eq!(probe_session_activity(&data).active_subagents, 1);

        // Gone quiet well past the window: no longer working.
        subagent_transcript(&test_path, SUB_ID, 600);
        assert_eq!(probe_session_activity(&data).active_subagents, 0);

    }

    /// The regression this replaces: a subagent reports to the parent many times while it
    /// works, and none of those messages means it is finished.
    #[test]
    fn test_probe_subagent_messages_do_not_retire_it() {
        let (_root, test_path, _tasks) = session("agy_probe_sub_msgs");
        {
            let mut f = File::create(&test_path).unwrap();
            writeln!(f, "{}", spawn_line(1, SUB_ID)).unwrap();
            // Three progress reports, exactly as a working subagent sends them.
            for step in 2..5 {
                writeln!(f, "{{\"step_index\":{},\"type\":\"SYSTEM_MESSAGE\",\"content\":\"[Message] sender={} priority=MESSAGE_PRIORITY_HIGH content=progress\"}}", step, SUB_ID).unwrap();
            }
        }
        subagent_transcript(&test_path, SUB_ID, 0);

        assert_eq!(probe_session_activity(&data_for(&test_path)).active_subagents, 1);

    }

    /// An explicit kill retires subagents at once, even while their transcripts are fresh.
    #[test]
    fn test_probe_subagent_kill_clears_indicator() {
        let (_root, test_path, _tasks) = session("agy_probe_sub_kill");
        {
            let mut f = File::create(&test_path).unwrap();
            writeln!(f, "{}", spawn_line(1, SUB_ID)).unwrap();
            writeln!(f, "{{\"step_index\":2,\"type\":\"GENERIC\",\"content\":\"Successfully killed 1 subagent(s) and their descendants.\\nKilled roles: Release Builder\"}}").unwrap();
        }
        subagent_transcript(&test_path, SUB_ID, 0);
        assert_eq!(probe_session_activity(&data_for(&test_path)).active_subagents, 0);

        // A subagent spawned after the kill is still tracked.
        {
            let mut f = File::options().append(true).open(&test_path).unwrap();
            writeln!(f, "{}", spawn_line(3, "aaaaaaaa-1111-2222-3333-444444444444")).unwrap();
        }
        subagent_transcript(&test_path, "aaaaaaaa-1111-2222-3333-444444444444", 0);
        assert_eq!(probe_session_activity(&data_for(&test_path)).active_subagents, 1);

    }

    /// Write a subagent transcript whose last step is `last`, aged `secs_ago` seconds.
    fn subagent_transcript_ending(parent_transcript: &Path, id: &str, last: &str, secs_ago: u64) {
        let brain = brain_dir(parent_transcript).unwrap();
        let logs = brain.join(id).join(".system_generated").join("logs");
        std::fs::create_dir_all(&logs).unwrap();
        let path = logs.join("transcript.jsonl");
        std::fs::write(
            &path,
            format!("{{\"step_index\":1,\"source\":\"USER_EXPLICIT\",\"type\":\"USER_INPUT\",\"status\":\"DONE\"}}\n{}\n", last),
        )
        .unwrap();
        let when = std::time::SystemTime::now() - std::time::Duration::from_secs(secs_ago);
        File::options().write(true).open(&path).unwrap().set_modified(when).unwrap();
    }

    /// A subagent that has answered and yielded is finished the moment it does so -- waiting
    /// out the idle window left the indicator lit for up to 90s after the work was over.
    #[test]
    fn test_probe_subagent_ended_turn_clears_immediately() {
        let (_root, test_path, _tasks) = session("agy_probe_sub_ended");
        {
            let mut f = File::create(&test_path).unwrap();
            writeln!(f, "{}", spawn_line(1, SUB_ID)).unwrap();
        }
        subagent_transcript_ending(
            &test_path,
            SUB_ID,
            "{\"step_index\":2,\"source\":\"MODEL\",\"type\":\"PLANNER_RESPONSE\",\"status\":\"DONE\",\"content\":\"The release process is complete.\"}",
            0,
        );

        assert_eq!(probe_session_activity(&data_for(&test_path)).active_subagents, 0);

    }

    /// A subagent whose last step is a tool call is mid-work, however long that call runs, and
    /// stays counted until the idle window expires.
    #[test]
    fn test_probe_subagent_mid_tool_call_stays_active() {
        let (_root, test_path, _tasks) = session("agy_probe_sub_tool");
        {
            let mut f = File::create(&test_path).unwrap();
            writeln!(f, "{}", spawn_line(1, SUB_ID)).unwrap();
        }
        let running = "{\"step_index\":2,\"source\":\"MODEL\",\"type\":\"PLANNER_RESPONSE\",\"status\":\"DONE\",\"tool_calls\":[{\"name\":\"run_command\"}]}";
        subagent_transcript_ending(&test_path, SUB_ID, running, 60);
        assert_eq!(probe_session_activity(&data_for(&test_path)).active_subagents, 1);

        subagent_transcript_ending(&test_path, SUB_ID, running, 600);
        assert_eq!(probe_session_activity(&data_for(&test_path)).active_subagents, 0);

    }

    /// A tail caught mid-write can end after `status` but before the `tool_calls` that follow
    /// it; an unterminated line is not evidence the turn is over.
    #[test]
    fn test_probe_subagent_partial_last_line_is_not_finished() {
        let (_root, test_path, _tasks) = session("agy_probe_sub_partial");
        {
            let mut f = File::create(&test_path).unwrap();
            writeln!(f, "{}", spawn_line(1, SUB_ID)).unwrap();
        }
        subagent_transcript_ending(
            &test_path,
            SUB_ID,
            "{\"step_index\":2,\"source\":\"MODEL\",\"type\":\"PLANNER_RESPONSE\",\"status\":\"DONE\",\"too",
            0,
        );

        assert_eq!(probe_session_activity(&data_for(&test_path)).active_subagents, 1);

    }

    /// A subagent the parent has just spawned has not written a transcript yet, and must
    /// still light the indicator rather than waiting for its first step.
    #[test]
    fn test_probe_freshly_spawned_subagent_counts() {
        let (_root, test_path, _tasks) = session("agy_probe_sub_fresh");
        {
            let mut f = File::create(&test_path).unwrap();
            writeln!(f, "{}", spawn_line(1, SUB_ID)).unwrap();
        }
        assert_eq!(probe_session_activity(&data_for(&test_path)).active_subagents, 1);

    }

    /// Viewing this file's own source — which contains the subagent marker inside a test
    /// literal — must not spawn phantom subagents.
    #[test]
    fn test_probe_ignores_non_uuid_subagent_ids() {
        let (_root, test_path, _tasks) = session("agy_probe_sub_prose");

        {
            let mut f = File::create(&test_path).unwrap();
            writeln!(f, "{{\"step_index\":1,\"type\":\"VIEW_FILE\",\"content\":\"...\\\\\\\"conversationId\\\\\\\":  \\\\\\\"sub-uuid-1\\\\\\\"...\"}}").unwrap();
        }

        assert_eq!(probe_session_activity(&data_for(&test_path)).active_subagents, 0);

    }

    #[test]
    fn test_uuid_shape() {
        assert!(is_uuid("6f1c2d34-88d5-4cb1-a308-20e52d1087e2"));
        assert!(!is_uuid("sub-uuid-1"));
        assert!(!is_uuid(""));
        assert!(!is_uuid("6f1c2d34-88d5-4cb1-a308-20e52d1087e2-extra"));
    }

    #[test]
    fn test_task_ref_shape() {
        assert_eq!(split_task_ref("sess/task-72"), Some((Some("sess"), "task-72")));
        assert_eq!(split_task_ref("task-7"), Some((None, "task-7")));
        assert_eq!(split_task_ref("<id>`"), None);
        assert_eq!(split_task_ref("task-"), None);
        assert_eq!(split_task_ref("task-abc"), None);
    }
}
