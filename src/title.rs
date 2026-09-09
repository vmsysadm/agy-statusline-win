use std::path::Path;

use crate::config::Config;
use crate::data::InputData;

/// Extract the leaf workspace folder name for terminal tab display.
pub(crate) fn get_workspace_name<'a>(data: &'a InputData, home_path: &str) -> &'a str {
    let raw_path = data
        .workspace
        .as_ref()
        .and_then(|w| w.project_dir.as_deref().or(w.current_dir.as_deref()))
        .or(data.cwd.as_deref())
        .unwrap_or("");

    if raw_path.is_empty() {
        return "agy";
    }

    if !home_path.is_empty() && raw_path == home_path {
        return "~";
    }

    let trimmed = raw_path.trim_end_matches(['\\', '/']);
    if trimmed.is_empty() {
        return raw_path;
    }

    Path::new(trimmed)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(raw_path)
}

use crate::probe::SessionActivity;

/// Full cycle of the working-state pulse, in milliseconds. Each half of the cycle draws one
/// brightness frame, so this is the dim -> bright -> dim round trip.
const PULSE_PERIOD_MS: u128 = 800;

/// Which brightness frame the working indicator should draw.
///
/// Unicode gives us the pair outright -- U+1F506 HIGH BRIGHTNESS and U+1F505 LOW BRIGHTNESS --
/// so the "glow" is a genuine brightness change rather than an approximation. It has to be a
/// glyph swap: OSC 0 titles are plain text and terminals ignore SGR colour inside them, so the
/// icon's actual colour is not ours to control.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Pulse {
    /// Do not animate; always draw the bright frame.
    Off,
    Bright,
    Dim,
}

/// Pick the current frame from the wall clock.
///
/// The statusline is a fresh process on every refresh, so it holds no state between frames;
/// the clock is the one phase source that needs none. The trade-off is that the title only
/// repaints when the CLI re-invokes us, so the pulse is *sampled* at the refresh rate -- it
/// reads as a rhythm when refreshes are frequent and regular, and as flicker when they are not.
pub(crate) fn pulse_from_clock(enabled: bool) -> Pulse {
    if !enabled {
        return Pulse::Off;
    }
    let ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    if (ms / (PULSE_PERIOD_MS / 2)) % 2 == 0 {
        Pulse::Bright
    } else {
        Pulse::Dim
    }
}

/// Choose an icon for the terminal tab title.
/// Base states:
/// - ✅ (\u{2705}) Ready / Done
/// - 🔆 (\u{1f506}) Working
/// - ❓ (\u{2753}) Waiting for User Input / Question
///
/// If background tasks or subagents are active, indicators are appended:
/// - ⚙️ (\u{2699}\u{fe0f}) Background task active
/// - 🤖 (\u{1f916}) Background subagent active
pub(crate) fn get_title_icon(data: &InputData, activity: &SessionActivity, pulse: Pulse) -> String {
    if activity.is_blocked_on_question {
        return "\u{2753}".to_string();
    }

    let state_raw = data.agent_state.as_deref().unwrap_or("idle");

    let base_icon = match state_raw {
        "idle" | "ready" | "review" | "reviewing" => "\u{2705}", // ✅ White heavy check mark in green box
        "waiting_for_input" | "waiting" => "\u{2753}",           // ❓ Red question mark (waiting for user answer/input)
        "thinking" => "\u{1f4ad}",                               // 💭 Thought balloon
        // Working alternates between the high- and low-brightness symbols so the tab glows
        // while this agy session is running. The glow tracks the session's own state, not
        // its subagents or background tasks -- those append their own indicators below, and
        // a session can be running with neither. Only this state animates: a steady icon is
        // what makes a tab scannable, and a resting session should stay still.
        "working" => match pulse {
            Pulse::Dim => "\u{1f505}",                           // 🔅 Low brightness
            Pulse::Off | Pulse::Bright => "\u{1f506}",           // 🔆 High brightness
        },
        "tool_use" | "tool" => "\u{2692}",                       // ⚒ Tools
        _ => "\u{223c}",                                         // ∼ Tilde
    };

    let mut out = base_icon.to_string();

    // If not blocked on question, append background task/subagent indicators
    if state_raw != "waiting_for_input" && state_raw != "waiting" {
        if activity.active_tasks > 0 {
            out.push_str("\u{2699}");          // ⚙ Single-codepoint gear (avoids double-glyph glitch in WezTerm)
        }
        if activity.active_subagents > 0 {
            out.push_str("\u{1f916}");         // 🤖 Robot
        }
    }

    out
}

/// Construct the OSC 0 escape sequence to update the terminal window/tab title.
pub(crate) fn build_terminal_title_sequence(
    data: &InputData,
    cfg: &Config,
    home_path: &str,
    activity: &SessionActivity,
) -> Option<String> {
    let title_cfg = cfg.terminal_title.as_ref();
    if !title_cfg.map(|t| t.enabled).unwrap_or(true) {
        return None;
    }

    let pulse = pulse_from_clock(title_cfg.map(|t| t.pulse).unwrap_or(true));
    let icon = get_title_icon(data, activity, pulse);
    let ws = get_workspace_name(data, home_path);

    // Use ECMA-48 String Terminator (ESC \) instead of BEL (\x07)
    // BEL triggers audio/visual tab notifications in WezTerm
    let mut seq = format!("\x1b]0;{} {}\x1b\\", icon, ws);

    if title_cfg.map(|t| t.osc7_cwd).unwrap_or(false) {
        if let Some(cwd) = data.cwd.as_deref() {
            let normalized = cwd.replace('\\', "/");
            seq.push_str(&format!("\x1b]7;file:///{}\x1b\\", normalized));
        }
    }

    Some(seq)
}

/// Directly emit an escape sequence to the active console buffer (`CONOUT$` on Windows / `/dev/tty` on Unix).
/// This bypasses parent process stdout capture (e.g. TUI frameworks like BubbleTea that pipe stdout into a cell buffer).
/// Falls back to stderr only when the console cannot be opened, and only if stderr is itself a terminal --
/// the host captures stderr for error reporting, so escape bytes must never leak into its logs.
///
/// The console is shared with every process attached to it, so a session that does not own
/// it stays silent -- see [`crate::session::owns_terminal_title`].
pub(crate) fn emit_title_to_terminal(seq: &str) {
    use std::fs::OpenOptions;
    use std::io::{IsTerminal, Write};

    if !crate::session::owns_terminal_title() {
        return;
    }

    let console = if cfg!(windows) { "CONOUT$" } else { "/dev/tty" };
    let written = OpenOptions::new()
        .write(true)
        .open(console)
        .map(|mut out| out.write_all(seq.as_bytes()).and_then(|_| out.flush()).is_ok())
        .unwrap_or(false);

    if written {
        return;
    }

    let mut err = std::io::stderr();
    if err.is_terminal() {
        let _ = err.write_all(seq.as_bytes());
        let _ = err.flush();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::WorkspaceInfo;
    use crate::probe::probe_session_activity;
    use std::path::PathBuf;

    /// Every session recorded under `tests/fixtures/` is replayed to its final step and the tab
    /// icon checked.
    ///
    /// The invariant is the whole bug class in one line: **a session whose work has finished must
    /// not still be wearing an indicator.** ⚙ (U+2699, background task) and 🤖 (U+1F916, subagent)
    /// are appended by [`get_title_icon`] from counts that only ever come down when the probe
    /// recognises a completion; every stuck-tab report so far has been a real transcript shape the
    /// probe failed to recognise. Capturing that session with `scripts/capture-fixture.ps1` adds a
    /// directory here and needs no new test code.
    ///
    /// Fixtures are copied to a scratch dir before use because the run mutates them: subagent
    /// transcript mtimes have to be re-stamped from `meta.json` ages. Committed timestamps are
    /// whatever `git checkout` wrote, and `subagent_is_working` measures age against a 90-second
    /// window -- read in place, a fixture would mean something different on every machine and
    /// every clone.
    #[test]
    fn recorded_sessions_end_with_a_clean_tab_icon() {
        let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests").join("fixtures");
        if !dir.exists() {
            return; // No corpus captured yet.
        }

        let mut checked = 0;
        for entry in std::fs::read_dir(&dir).unwrap() {
            let fixture = entry.unwrap().path();
            if !fixture.join("meta.json").exists() {
                continue;
            }
            check_fixture(&fixture);
            checked += 1;
        }
        eprintln!("replayed {} recorded session(s)", checked);
    }

    fn check_fixture(fixture: &Path) {
        let name = fixture.file_name().unwrap().to_string_lossy().into_owned();
        let meta: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(fixture.join("meta.json")).unwrap())
                .unwrap_or_else(|e| panic!("{}: unreadable meta.json: {}", name, e));

        let conv = meta["conversation"].as_str().unwrap();
        let scratch = crate::probe::tests::scratch_dir(&format!("fixture-{}", name));
        copy_tree(fixture, scratch.path());

        // Re-apply each subagent's recorded age, so the 90s liveness window means the same thing
        // here as it did in the session that was captured.
        if let Some(ages) = meta["subagent_ages"].as_object() {
            for (id, secs) in ages {
                let path = scratch
                    .path()
                    .join(id)
                    .join(".system_generated")
                    .join("logs")
                    .join("transcript.jsonl");
                if let Ok(f) = std::fs::File::options().write(true).open(&path) {
                    let when = std::time::SystemTime::now()
                        - std::time::Duration::from_secs(secs.as_u64().unwrap_or(0));
                    let _ = f.set_modified(when);
                }
            }
        }

        let transcript = scratch
            .path()
            .join(conv)
            .join(".system_generated")
            .join("logs")
            .join("transcript.jsonl");
        assert!(transcript.exists(), "{}: no transcript for conversation {}", name, conv);

        let mut data = InputData::default();
        data.transcript_path = Some(transcript.to_string_lossy().into_owned());
        let activity = probe_session_activity(&data);
        let icon = get_title_icon(&data, &activity, Pulse::Off);

        match meta["expect"].as_str().unwrap_or("clean") {
            "clean" => assert!(
                !icon.contains('\u{2699}') && !icon.contains('\u{1f916}'),
                "{}: tab icon still wearing an indicator at end of session: {:?} \
                 (tasks={}, subagents={}). {}",
                name,
                icon,
                activity.active_tasks,
                activity.active_subagents,
                meta["note"].as_str().unwrap_or(""),
            ),
            "stuck" => assert!(
                icon.contains('\u{2699}') || icon.contains('\u{1f916}'),
                "{}: expected an active indicator, got {:?}",
                name,
                icon,
            ),
            other => panic!("{}: unknown expect value {:?}", name, other),
        }
    }

    fn copy_tree(src: &Path, dst: &Path) {
        std::fs::create_dir_all(dst).unwrap();
        for entry in std::fs::read_dir(src).unwrap() {
            let entry = entry.unwrap();
            let to = dst.join(entry.file_name());
            if entry.file_type().unwrap().is_dir() {
                copy_tree(&entry.path(), &to);
            } else {
                std::fs::copy(entry.path(), &to).unwrap();
            }
        }
    }

    /// A stand-in home directory, built from the platform's own temp location.
    ///
    /// Nothing below cares *what* the home path is -- `get_workspace_name` compares it against
    /// `cwd` and takes the leaf, so the assertions hold for any absolute path. Deriving it keeps
    /// the drive letter, the separator and the shape of a home directory out of the test, all of
    /// which are the operating system's business rather than this function's.
    fn fake_home() -> String {
        std::env::temp_dir().join("agy-home").to_string_lossy().into_owned()
    }

    /// `<fake_home>/<rest...>`, so callers never spell out a separator.
    fn under_home(rest: &[&str]) -> String {
        let mut p = std::env::temp_dir().join("agy-home");
        for part in rest {
            p = p.join(part);
        }
        p.to_string_lossy().into_owned()
    }

    #[test]
    fn test_get_workspace_name() {
        let home = fake_home();
        let mut data = InputData::default();
        assert_eq!(get_workspace_name(&data, &home), "agy");

        data.cwd = Some(under_home(&["code", "agy-statusline-win"]));
        assert_eq!(get_workspace_name(&data, &home), "agy-statusline-win");

        // A trailing separator must not become the leaf.
        data.cwd = Some(format!("{}{}", under_home(&["code", "agy-statusline-win"]), std::path::MAIN_SEPARATOR));
        assert_eq!(get_workspace_name(&data, &home), "agy-statusline-win");

        // A workspace outside the home directory entirely takes precedence over cwd.
        data.workspace = Some(WorkspaceInfo {
            project_dir: Some(
                std::env::temp_dir().join("elsewhere").join("my-project").to_string_lossy().into_owned(),
            ),
            current_dir: None,
        });
        assert_eq!(get_workspace_name(&data, &home), "my-project");

        data.workspace = None;
        data.cwd = Some(home.clone());
        assert_eq!(get_workspace_name(&data, &home), "~");
    }

    #[test]
    fn test_build_terminal_title_sequence() {
        let mut data = InputData::default();
        data.cwd = Some(under_home(&["code", "agy-statusline-win"]));
        data.agent_state = Some("review".to_string());

        let mut cfg = Config::default();
        // Pin the bright frame so these assertions do not depend on the wall clock.
        cfg.terminal_title = Some(crate::config::TerminalTitleConfig { enabled: true, osc7_cwd: false, pulse: false });
        let mut activity = SessionActivity::default();

        let seq = build_terminal_title_sequence(&data, &cfg, &fake_home(), &activity).unwrap();
        // Uses ✅ (\u{2705}) check mark
        assert_eq!(seq, "\x1b]0;\u{2705} agy-statusline-win\x1b\\");

        // With active background task
        activity.active_tasks = 1;
        let seq = build_terminal_title_sequence(&data, &cfg, &fake_home(), &activity).unwrap();
        assert_eq!(seq, "\x1b]0;\u{2705}\u{2699} agy-statusline-win\x1b\\");

        // With both task and subagent
        activity.active_subagents = 1;
        let seq = build_terminal_title_sequence(&data, &cfg, &fake_home(), &activity).unwrap();
        assert_eq!(seq, "\x1b]0;\u{2705}\u{2699}\u{1f916} agy-statusline-win\x1b\\");

        // Working state with background jobs
        data.agent_state = Some("working".to_string());
        let seq = build_terminal_title_sequence(&data, &cfg, &fake_home(), &activity).unwrap();
        assert_eq!(seq, "\x1b]0;\u{1f506}\u{2699}\u{1f916} agy-statusline-win\x1b\\");

        data.agent_state = Some("waiting_for_input".to_string());
        let seq = build_terminal_title_sequence(&data, &cfg, &fake_home(), &activity).unwrap();
        // Uses ❓ (\u{2753}) red question mark when waiting for user input
        assert_eq!(seq, "\x1b]0;\u{2753} agy-statusline-win\x1b\\");
    }

    /// The working icon swaps to the low-brightness symbol on the dim frame, and only for
    /// the working state -- every other state stays steady.
    #[test]
    fn test_working_icon_pulses() {
        let mut data = InputData::default();
        let act = SessionActivity::default();

        data.agent_state = Some("working".to_string());
        assert_eq!(get_title_icon(&data, &act, Pulse::Bright), "\u{1f506}");
        assert_eq!(get_title_icon(&data, &act, Pulse::Dim), "\u{1f505}");
        assert_eq!(get_title_icon(&data, &act, Pulse::Off), "\u{1f506}");

        // A resting session must not flicker.
        data.agent_state = Some("idle".to_string());
        assert_eq!(get_title_icon(&data, &act, Pulse::Dim), "\u{2705}");
        data.agent_state = Some("thinking".to_string());
        assert_eq!(get_title_icon(&data, &act, Pulse::Dim), "\u{1f4ad}");
    }

    /// Background-task indicators must survive the dim frame.
    #[test]
    fn test_pulse_keeps_task_indicators() {
        let mut data = InputData::default();
        data.agent_state = Some("working".to_string());
        let act = SessionActivity { is_blocked_on_question: false, active_tasks: 1, active_subagents: 1 };
        assert_eq!(get_title_icon(&data, &act, Pulse::Dim), "\u{1f505}\u{2699}\u{1f916}");
    }

    /// Disabling the pulse pins the bright frame; enabling it yields one of the two.
    #[test]
    fn test_pulse_from_clock() {
        assert_eq!(pulse_from_clock(false), Pulse::Off);
        assert!(matches!(pulse_from_clock(true), Pulse::Bright | Pulse::Dim));
    }

    #[test]
    fn test_terminal_title_disabled() {
        let mut data = InputData::default();
        data.cwd = Some(under_home(&["code", "agy-statusline-win"]));

        let mut cfg = Config::default();
        cfg.terminal_title = Some(crate::config::TerminalTitleConfig {
            enabled: false,
            osc7_cwd: false,
            pulse: true,
        });

        let seq = build_terminal_title_sequence(&data, &cfg, &fake_home(), &SessionActivity::default());
        assert!(seq.is_none());
    }
}
