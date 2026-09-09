mod ansi;
mod config;
mod data;
mod layout;
mod probe;
mod procinfo;
mod segments;
mod session;
mod theme;
mod title;
mod yolo;

use std::env;
use std::io::{self, BufWriter, Read, Write};

/// One-line JSON report of the tab icon this payload resolves to, for `--print-title`.
///
/// The pulse is frozen to [`title::Pulse::Off`] rather than sampled from the clock: the working
/// icon otherwise alternates 🔆/🔅 twice a second, so consecutive samples of an unchanged session
/// would differ for reasons that have nothing to do with what is being diagnosed.
///
/// `codepoints` is not redundant with `icon`. The indicators are single-codepoint by design --
/// U+2699 without the VS16 that makes WezTerm draw a double-width glyph -- and a regression back
/// to the emoji-presentation form is invisible in a log that carries only the rendered character.
fn report_title(
    data: &data::InputData,
    cfg: &config::Config,
    home_path: &str,
    activity: &probe::SessionActivity,
) -> String {
    let icon = title::get_title_icon(data, activity, title::Pulse::Off);
    let codepoints: Vec<String> = icon.chars().map(|c| format!("U+{:04X}", c as u32)).collect();

    serde_json::json!({
        "icon": icon,
        "codepoints": codepoints,
        "tasks": activity.active_tasks,
        "subagents": activity.active_subagents,
        "blocked": activity.is_blocked_on_question,
        "state": data.agent_state.as_deref().unwrap_or("idle"),
        "workspace": title::get_workspace_name(data, home_path),
        // A session with titles switched off emits nothing to the console, so the tab stays
        // frozen no matter what this icon says. Surface it or the watcher looks broken.
        "title_enabled": cfg.terminal_title.as_ref().map(|t| t.enabled).unwrap_or(true),
    })
    .to_string()
}

fn main() {
    let mut input = String::new();
    let _ = io::stdin().read_to_string(&mut input);

    // Single-pass JSON: parse once, detect YOLO, then consume
    let raw_val: serde_json::Value = serde_json::from_str(&input).unwrap_or_default();
    let yolo_from_json = yolo::detect_yolo_in_json(&raw_val);
    let mut data: data::InputData = serde_json::from_value(raw_val).unwrap_or_default();

    // Probe real-time session activity (question prompts, active background tasks, active subagents)
    let activity = probe::probe_session_activity(&data);
    if activity.is_blocked_on_question {
        data.agent_state = Some("waiting_for_input".to_string());
    }

    let home_path = env::var("USERPROFILE").or_else(|_| env::var("HOME")).unwrap_or_default();
    let cfg = config::load_config(&home_path);

    let stdout = io::stdout();
    let mut handle = BufWriter::new(stdout.lock());

    // Diagnostic mode: report what the tab icon *would* be, on stdout, and stop. Used by the
    // title-watching scripts to turn the tab into a readable timeline -- the real emit goes to
    // CONOUT$, where nothing can capture it. See `report_title` for why the pulse is frozen.
    if env::args().any(|a| a == "--print-title") {
        let _ = writeln!(handle, "{}", report_title(&data, &cfg, &home_path, &activity));
        return;
    }

    // Emit terminal tab title escape sequence (OSC 0 / OSC 7) directly to console
    if let Some(title_seq) = title::build_terminal_title_sequence(&data, &cfg, &home_path, &activity) {
        title::emit_title_to_terminal(&title_seq);
    }

    let theme = theme::Theme::new(&data, &cfg);
    let output_lines = layout::render_statusline(&data, &cfg, &theme, yolo_from_json, &home_path);

    for line in output_lines {
        let _ = writeln!(handle, "{}", line);
    }
}
