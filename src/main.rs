use serde::Deserialize;
use std::collections::HashMap;
use std::env;
use std::fs;
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use unicode_width::UnicodeWidthStr;

#[derive(Deserialize, Debug, Default)]
#[serde(rename_all = "camelCase")]
struct ContextWindow {
    #[serde(default, alias = "used_percentage")]
    used_percentage: Option<f64>,
    #[serde(default, alias = "total_input_tokens")]
    total_input_tokens: Option<u64>,
    #[serde(default, alias = "total_output_tokens")]
    total_output_tokens: Option<u64>,
    #[serde(default, alias = "context_window_size")]
    context_window_size: Option<u64>,
}

#[derive(Deserialize, Debug, Default)]
#[serde(rename_all = "camelCase")]
struct Sandbox {
    #[serde(default)]
    enabled: Option<bool>,
    #[serde(default, alias = "allow_network")]
    allow_network: Option<bool>,
}

#[derive(Deserialize, Debug, Default)]
#[serde(rename_all = "camelCase")]
struct Model {
    #[serde(default)]
    id: Option<String>,
    #[serde(default, alias = "display_name")]
    display_name: Option<String>,
    #[serde(default)]
    effort: Option<String>,
    #[serde(default, alias = "effort_level")]
    effort_level: Option<String>,
}

#[derive(Deserialize, Debug, Default, Clone)]
#[serde(rename_all = "camelCase")]
struct QuotaEntry {
    #[serde(default, alias = "remaining_fraction")]
    remaining_fraction: Option<f64>,
    #[serde(default, alias = "reset_in_seconds")]
    reset_in_seconds: Option<u64>,
}

#[derive(Deserialize, Debug, Default)]
#[serde(rename_all = "camelCase")]
struct InputData {
    #[serde(default, alias = "agent_state")]
    agent_state: Option<String>,
    #[serde(default, alias = "context_window")]
    context_window: Option<ContextWindow>,
    #[serde(default)]
    sandbox: Option<Sandbox>,
    #[serde(default, alias = "artifact_count")]
    artifact_count: Option<u64>,
    #[serde(default, alias = "task_count")]
    task_count: Option<u64>,
    #[serde(default)]
    subagents: Option<serde_json::Value>,
    #[serde(default)]
    model: Option<Model>,
    #[serde(default, alias = "terminal_width")]
    terminal_width: Option<usize>,
    #[serde(default)]
    cwd: Option<String>,
    #[serde(default, alias = "conversation_id")]
    conversation_id: Option<String>,
    #[serde(default, alias = "cycle_mode")]
    cycle_mode: Option<String>,
    #[serde(default, alias = "nerd_fonts_supported")]
    nerd_fonts_supported: Option<bool>,
    #[serde(default)]
    quota: Option<HashMap<String, QuotaEntry>>,
    #[serde(default, alias = "auto_approve")]
    auto_approve: Option<bool>,
    #[serde(default, alias = "auto_approve_enabled")]
    auto_approve_enabled: Option<bool>,
    #[serde(default, alias = "dangerously_skip_permissions")]
    dangerously_skip_permissions: Option<bool>,
    #[serde(default, alias = "skip_permissions")]
    skip_permissions: Option<bool>,
    #[serde(default, alias = "approval_mode")]
    approval_mode: Option<String>,
    #[serde(default)]
    mode: Option<String>,
    #[serde(default, alias = "use_ascii")]
    use_ascii: Option<bool>,
    #[serde(default)]
    yolo: Option<bool>,
    #[serde(default, alias = "is_yolo")]
    is_yolo: Option<bool>,
}

#[derive(Deserialize, Debug, Default)]
struct ConfigColors {
    reset: Option<String>,
    bold: Option<String>,
    dim: Option<String>,
    italic: Option<String>,
    foreground: Option<HashMap<String, String>>,
    ui: Option<HashMap<String, String>>,
}


#[derive(Deserialize, Debug, Default)]
#[serde(rename_all = "camelCase")]
struct Config {
    colors: Option<ConfigColors>,
    icons: Option<serde_json::Value>,
    #[serde(default, alias = "show_yolo")]
    show_yolo: Option<bool>,
    #[serde(default, alias = "always_show_yolo")]
    always_show_yolo: Option<bool>,
    #[serde(default)]
    yolo: Option<bool>,
    #[serde(default, alias = "use_ascii")]
    use_ascii: Option<bool>,
}

fn convert_color_value(val: &str) -> String {
    if val.is_empty() {
        return val.to_string();
    }
    let mut result = String::with_capacity(val.len() * 2);
    let chars: Vec<char> = val.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        if chars[i] == '#' {
            if i + 6 < chars.len() && chars[i + 1..=i + 6].iter().all(|c| c.is_ascii_hexdigit()) {
                let hex: String = chars[i + 1..=i + 6].iter().collect();
                if let (Ok(r), Ok(g), Ok(b)) = (
                    u8::from_str_radix(&hex[0..2], 16),
                    u8::from_str_radix(&hex[2..4], 16),
                    u8::from_str_radix(&hex[4..6], 16),
                ) {
                    result.push_str(&format!("\x1b[38;2;{};{};{}m", r, g, b));
                    i += 7;
                    continue;
                }
            } else if i + 3 < chars.len() && chars[i + 1..=i + 3].iter().all(|c| c.is_ascii_hexdigit()) {
                let hex: String = chars[i + 1..=i + 3].iter().collect();
                let r_str = format!("{}{}", &hex[0..1], &hex[0..1]);
                let g_str = format!("{}{}", &hex[1..2], &hex[1..2]);
                let b_str = format!("{}{}", &hex[2..3], &hex[2..3]);
                if let (Ok(r), Ok(g), Ok(b)) = (
                    u8::from_str_radix(&r_str, 16),
                    u8::from_str_radix(&g_str, 16),
                    u8::from_str_radix(&b_str, 16),
                ) {
                    result.push_str(&format!("\x1b[38;2;{};{};{}m", r, g, b));
                    i += 4;
                    continue;
                }
            }
        }
        result.push(chars[i]);
        i += 1;
    }

    result
}

fn get_icon_str(
    active_icon_set: &Option<serde_json::Value>,
    raw_icons_cfg: &Option<serde_json::Value>,
    category: &str,
    key: &str,
    default_str: &str,
) -> String {
    let keys_to_try: Vec<&str> = match key {
        "ready" => vec!["ready", "idle"],
        "branch" => vec!["branch", "git"],
        "background_task" => vec!["background_task", "tasks", "task"],
        "subagent" => vec!["subagent", "subagents", "agents"],
        "artifact" => vec!["artifact", "art"],
        "context" => vec!["context", "ctx"],
        "token" => vec!["token", "tok"],
        "conversation" => vec!["conversation", "conv", "id"],
        "folder" => vec!["folder", "dir"],
        _ => vec![key],
    };

    for k in keys_to_try {
        if let Some(val) = active_icon_set {
            if let Some(cat) = val.get(category) {
                if let Some(s) = cat.get(k).and_then(|v| v.as_str()) {
                    return s.to_string();
                }
            }
            if let Some(s) = val.get(k).and_then(|v| v.as_str()) {
                return s.to_string();
            }
        }
        if let Some(val) = raw_icons_cfg {
            if let Some(cat) = val.get(category) {
                if let Some(s) = cat.get(k).and_then(|v| v.as_str()) {
                    return s.to_string();
                }
            }
            if let Some(s) = val.get(k).and_then(|v| v.as_str()) {
                return s.to_string();
            }
        }
    }
    default_str.to_string()
}

fn get_git_branch(cwd: &str) -> Option<String> {
    if cwd.is_empty() {
        return None;
    }
    let git_head = Path::new(cwd).join(".git").join("HEAD");
    if let Ok(content) = fs::read_to_string(&git_head) {
        let trimmed = content.trim();
        if trimmed.starts_with("ref: refs/heads/") {
            return Some(trimmed[16..].to_string());
        } else if trimmed.len() >= 7 {
            return Some(trimmed[..7].to_string());
        }
    }
    None
}

fn strip_ansi(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut in_esc = false;
    for c in s.chars() {
        if c == '\x1b' {
            in_esc = true;
        } else if in_esc {
            if c == 'm' {
                in_esc = false;
            }
        } else {
            out.push(c);
        }
    }
    out
}

fn visible_len(s: &str) -> usize {
    let clean = strip_ansi(s);
    UnicodeWidthStr::width(clean.as_str())
}

fn format_human(num: u64) -> String {
    if num >= 1_000_000 {
        format!("{:.1}M", num as f64 / 1_000_000.0)
    } else if num >= 1_000 {
        format!("{:.1}K", num as f64 / 1_000.0)
    } else {
        num.to_string()
    }
}

fn format_seconds(s: u64) -> String {
    if s == 0 {
        return "0s".to_string();
    }
    if s >= 3600 {
        let h = s / 3600;
        let m = (s % 3600) / 60;
        format!("{}h{}m", h, m)
    } else if s >= 60 {
        let m = s / 60;
        format!("{}m", m)
    } else {
        format!("{}s", s)
    }
}

fn get_tokens(s: &str) -> Vec<String> {
    s.to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .filter(|t| !t.is_empty())
        .map(|t| t.to_string())
        .collect()
}

#[derive(Clone, Debug)]
struct MatchedQuota {
    key: String,
    remaining_fraction: f64,
    reset_in_seconds: u64,
    score: f64,
}

fn match_quotas(data: &InputData, model_disp: &str) -> Vec<MatchedQuota> {
    let mut matched_quotas = Vec::new();
    let quota_map = match &data.quota {
        Some(m) if !m.is_empty() => m,
        _ => return matched_quotas,
    };

    let model_tokens = get_tokens(model_disp);
    let mut all_entries = Vec::new();
    let tp_keywords = ["claude", "gpt", "opus", "sonnet", "haiku", "o1", "o3", "deepseek"];

    for (key, q_val) in quota_map {
        if let Some(rem) = q_val.remaining_fraction {
            let key_tokens = get_tokens(key);
            let mut matches = 0;

            for t in &key_tokens {
                if model_tokens.contains(t) {
                    matches += 1;
                } else if t == "3p" {
                    if model_tokens.iter().any(|mt| tp_keywords.contains(&mt.as_str())) {
                        matches += 1;
                    }
                }
            }

            let mut score = 0.0;
            if matches >= 1 {
                score = (matches as f64 * 100.0) + (matches as f64 / key_tokens.len() as f64);
                let key_lower = key.to_lowercase();
                if key_lower.contains("5h") || key_lower.contains("five") {
                    score += 10.0;
                }
            }

            all_entries.push(MatchedQuota {
                key: key.clone(),
                remaining_fraction: rem,
                reset_in_seconds: q_val.reset_in_seconds.unwrap_or(0),
                score,
            });
        }
    }

    if !all_entries.is_empty() {
        let mut matched: Vec<_> = all_entries.iter().filter(|e| e.score > 0.0).cloned().collect();
        if !matched.is_empty() {
            matched.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
            matched_quotas = matched;
        } else {
            all_entries.sort_by(|a, b| a.remaining_fraction.partial_cmp(&b.remaining_fraction).unwrap_or(std::cmp::Ordering::Equal));
            matched_quotas.push(all_entries[0].clone());
        }
    }

    matched_quotas
}

fn make_bar(pct: f64, len: usize, fill_color: &str, fg_gray: &str, reset: &str, use_ascii: bool) -> String {
    let pct_int = pct.clamp(0.0, 100.0);
    let filled = ((pct_int * len as f64) / 100.0).floor() as usize;

    if use_ascii {
        let empty = len.saturating_sub(filled);
        format!("[{}{}{}{}]", fill_color, "=".repeat(filled), reset, " ".repeat(empty))
    } else {
        let remainder = ((pct_int * len as f64) % 100.0).floor() as usize;

        let block_full = '\u{2588}';
        let block_dark = '\u{2593}';
        let block_med = '\u{2592}';
        let block_light = '\u{2591}';

        let mut bar = String::new();
        for i in 0..len {
            if i < filled {
                bar.push_str(&format!("{}{}{}", fill_color, block_full, reset));
            } else if i == filled {
                if remainder >= 75 {
                    bar.push_str(&format!("{}{}{}{}", fill_color, block_dark, reset, fg_gray));
                } else if remainder >= 50 {
                    bar.push_str(&format!("{}{}{}{}", fill_color, block_med, reset, fg_gray));
                } else {
                    bar.push_str(&format!("{}{}{}{}", fill_color, block_light, reset, fg_gray));
                }
            } else {
                bar.push_str(&format!("{}{}{}", fg_gray, block_light, reset));
            }
        }
        bar
    }
}

fn format_single_quota(
    entry: &MatchedQuota,
    mode: &str,
    show_icon: bool,
    icon_unknown: &str,
    fg_cyan: &str,
    fg_bright_cyan: &str,
    fg_bright_red: &str,
    fg_gray: &str,
    num_color: &str,
    reset: &str,
    use_ascii: bool,
) -> String {
    let pct = (entry.remaining_fraction * 100.0).round() as u64;
    let q_reset = format_seconds(entry.reset_in_seconds);

    let clean_name = entry
        .key
        .trim_start_matches("gemini-")
        .trim_start_matches("3p-");
    let clean_name = match clean_name {
        "5h" => "5h",
        "weekly" => "wk",
        other => other,
    };

    let q_color = if pct <= 10 {
        fg_bright_red
    } else if pct <= 40 {
        fg_bright_cyan
    } else {
        fg_cyan
    };

    let icon_str = if show_icon {
        if icon_unknown.is_empty() {
            String::new()
        } else {
            format!("{}{}{}  ", fg_cyan, icon_unknown, reset)
        }
    } else {
        String::new()
    };

    if mode == "narrow" {
        return format!("{}{}{}%{} {}{}{}", icon_str, num_color, pct, reset, fg_gray, clean_name, reset);
    }

    if mode == "med" {
        return format!(
            "{}{}{}%{} {}{}{} {}{}{}",
            icon_str, num_color, pct, reset, fg_gray, clean_name, reset, fg_gray, q_reset, reset
        );
    }

    let bar = make_bar(pct as f64, 5, q_color, fg_gray, reset, use_ascii);
    format!(
        "{}{}{}{}%{} {}{}{} {}{}{}",
        icon_str, bar, num_color, pct, reset, fg_gray, clean_name, reset, fg_gray, q_reset, reset
    )
}

fn format_quota(
    matched_quotas: &[MatchedQuota],
    mode: &str,
    icon_unknown: &str,
    fg_cyan: &str,
    fg_bright_cyan: &str,
    fg_bright_red: &str,
    fg_gray: &str,
    num_color: &str,
    reset: &str,
    use_ascii: bool,
) -> String {
    if matched_quotas.is_empty() {
        return String::new();
    }

    if mode == "narrow" {
        return format_single_quota(
            &matched_quotas[0],
            mode,
            true,
            icon_unknown,
            fg_cyan,
            fg_bright_cyan,
            fg_bright_red,
            fg_gray,
            num_color,
            reset,
            use_ascii,
        );
    }

    let mut parts = Vec::new();
    let mut first = true;
    for entry in matched_quotas {
        parts.push(format_single_quota(
            entry,
            mode,
            first,
            icon_unknown,
            fg_cyan,
            fg_bright_cyan,
            fg_bright_red,
            fg_gray,
            num_color,
            reset,
            use_ascii,
        ));
        first = false;
    }

    parts.join(" ")
}

fn get_shortened_path(path: &str, max_len: usize, home_path: &str) -> String {
    if path.is_empty() {
        return String::new();
    }
    let mut short_path = path.to_string();
    if !home_path.is_empty() && path.starts_with(home_path) {
        short_path = format!("~{}", &path[home_path.len()..]);
    }

    if max_len == 0 {
        if short_path == "~" {
            "~".to_string()
        } else {
            Path::new(path)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or(path)
                .to_string()
        }
    } else if short_path.chars().count() > max_len {
        let leaf = Path::new(path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(path);
        format!("...{}", leaf)
    } else {
        short_path
    }
}

fn format_branch(branch: Option<&str>, max_len: usize, fg_blue: &str, icon_branch: &str, reset: &str) -> String {
    if let Some(name) = branch {
        let mut b_name = name.to_string();
        if max_len > 0 && b_name.chars().count() > max_len {
            b_name = format!("{}..", &b_name[..max_len]);
        }
        format!("{}{}{} {}{}", fg_blue, icon_branch, reset, b_name, reset)
    } else {
        String::new()
    }
}

fn format_sandbox(
    sandbox: &Sandbox,
    mode: &str,
    icon_net: &str,
    icon_no_net: &str,
    icon_off: &str,
    fg_green: &str,
    fg_bright_green: &str,
    fg_red: &str,
    fg_bright_red: &str,
    bold: &str,
    reset: &str,
) -> String {
    let enabled = sandbox.enabled.unwrap_or(false);
    let net = sandbox.allow_network.unwrap_or(false);

    if enabled {
        let icon = if net { icon_net } else { icon_no_net };
        if mode == "wide" {
            let label = if net { "ON (net)" } else { "ON (no-net)" };
            format!("{}{}{} {}{}{}{}", fg_green, icon, reset, fg_bright_green, bold, label, reset)
        } else if mode == "med" {
            format!("{}{}{} {}{}ON{}", fg_green, icon, reset, fg_bright_green, bold, reset)
        } else {
            format!("{}{}{}", fg_green, icon, reset)
        }
    } else {
        if mode == "wide" || mode == "med" {
            format!("{}{}{} {}{}OFF{}", fg_red, icon_off, reset, fg_bright_red, bold, reset)
        } else {
            format!("{}{}{}", fg_red, icon_off, reset)
        }
    }
}

fn print_right_aligned(left: &str, right: &str, total_cols: usize) -> String {
    let left_vis = visible_len(left);
    let right_vis = visible_len(right);

    let pad = if total_cols > left_vis + right_vis {
        total_cols - left_vis - right_vis
    } else {
        1
    };

    format!("{}{}{}", left, " ".repeat(pad), right)
}

fn join_with_dot(items: &[String], dot: &str) -> String {
    items.iter().filter(|s| !s.is_empty()).cloned().collect::<Vec<_>>().join(dot)
}

fn join_with_space(items: &[String]) -> String {
    items.iter().filter(|s| !s.is_empty()).cloned().collect::<Vec<_>>().join("  ")
}

fn detect_yolo_in_json(v: &serde_json::Value) -> bool {
    match v {
        serde_json::Value::Object(map) => {
            for (key, val) in map {
                let k_lower = key.to_lowercase();

                if k_lower == "sandbox" || k_lower == "cwd" || k_lower == "conversation_id" || k_lower == "conversationid" {
                    continue;
                }

                let is_yolo_key = k_lower.contains("yolo")
                    || k_lower.contains("dangerously")
                    || k_lower.contains("skippermission")
                    || k_lower.contains("skip_permission")
                    || k_lower.contains("autoapprove")
                    || k_lower.contains("auto_approve")
                    || k_lower.contains("approval")
                    || k_lower.contains("permission")
                    || k_lower == "mode";

                if is_yolo_key {
                    match val {
                        serde_json::Value::Bool(b) => {
                            if *b {
                                return true;
                            }
                        }
                        serde_json::Value::String(s) => {
                            let s_lower = s.to_lowercase();
                            if s_lower == "yolo"
                                || s_lower == "auto_approve"
                                || s_lower == "auto-approve"
                                || s_lower == "autoapprove"
                                || s_lower == "skip"
                                || s_lower == "true"
                                || s_lower == "enabled"
                            {
                                return true;
                            }
                        }
                        _ => {}
                    }
                } else if let serde_json::Value::String(s) = val {
                    let s_lower = s.to_lowercase();
                    if s_lower == "yolo" || s_lower == "auto_approve" || s_lower == "auto-approve" {
                        return true;
                    }
                }

                if detect_yolo_in_json(val) {
                    return true;
                }
            }
            false
        }
        serde_json::Value::Array(arr) => {
            for item in arr {
                if let serde_json::Value::String(s) = item {
                    let s_lower = s.to_lowercase();
                    if s_lower.contains("dangerously") || s_lower.contains("yolo") || s_lower.contains("skip_permission") {
                        return true;
                    }
                }
                if detect_yolo_in_json(item) {
                    return true;
                }
            }
            false
        }
        _ => false,
    }
}

fn check_process_cmdline_for_yolo() -> bool {
    let mut sys = sysinfo::System::new();
    sys.refresh_processes_specifics(
        sysinfo::ProcessRefreshKind::new().with_cmd(sysinfo::UpdateKind::Always),
    );

    // Walk up the parent process chain from our own PID
    let mut current_pid = sysinfo::Pid::from_u32(std::process::id());
    // Limit depth to prevent infinite loops from circular parent references
    for _ in 0..20 {
        let process = match sys.process(current_pid) {
            Some(p) => p,
            None => break,
        };
        let name = process.name().to_lowercase();
        if name.contains("node") || name.contains("agy") || name.contains("antigravity") {
            for arg in process.cmd() {
                let arg_lower = arg.to_lowercase();
                if arg_lower.contains("dangerously")
                    || arg_lower.contains("skip-permissions")
                    || arg_lower.contains("skippermissions")
                {
                    return true;
                }
            }
        }
        current_pid = match process.parent() {
            Some(ppid) => ppid,
            None => break,
        };
    }
    false
}

fn main() {
    let mut input = String::new();
    let _ = io::stdin().read_to_string(&mut input);

    let raw_val: serde_json::Value = serde_json::from_str(&input).unwrap_or(serde_json::Value::Null);
    let data: InputData = serde_json::from_value(raw_val.clone()).unwrap_or_default();

    let home_path = env::var("USERPROFILE").or_else(|_| env::var("HOME")).unwrap_or_default();
    let config_path = PathBuf::from(&home_path).join(".gemini").join("antigravity-cli").join("statusline_config.json");

    let mut cfg: Config = Config::default();
    if config_path.exists() {
        if let Ok(cfg_content) = fs::read_to_string(&config_path) {
            cfg = serde_json::from_str(&cfg_content).unwrap_or_default();
        }
    }

    let reset = cfg.colors.as_ref().and_then(|c| c.reset.as_deref()).map(convert_color_value).unwrap_or_else(|| "\x1b[0m".to_string());
    let bold = cfg.colors.as_ref().and_then(|c| c.bold.as_deref()).map(convert_color_value).unwrap_or_else(|| "\x1b[1m".to_string());
    let _dim = cfg.colors.as_ref().and_then(|c| c.dim.as_deref()).map(convert_color_value).unwrap_or_else(|| "\x1b[2m".to_string());
    let italic = cfg.colors.as_ref().and_then(|c| c.italic.as_deref()).map(convert_color_value).unwrap_or_else(|| "\x1b[3m".to_string());

    let get_fg = |key: &str, default_val: &str| -> String {
        cfg.colors
            .as_ref()
            .and_then(|c| c.foreground.as_ref())
            .and_then(|fg| fg.get(key))
            .map(|v| convert_color_value(v))
            .unwrap_or_else(|| default_val.to_string())
    };

    let fg_red = get_fg("red", "\x1b[31m");
    let fg_green = get_fg("green", "\x1b[32m");
    let fg_yellow = get_fg("yellow", "\x1b[33m");
    let fg_blue = get_fg("blue", "\x1b[34m");
    let fg_magenta = get_fg("magenta", "\x1b[35m");
    let fg_cyan = get_fg("cyan", "\x1b[36m");
    let fg_white = get_fg("white", "\x1b[37m");
    let fg_gray = get_fg("gray", "\x1b[90m");

    let fg_bright_red = get_fg("bright_red", "\x1b[91m");
    let fg_bright_green = get_fg("bright_green", "\x1b[92m");
    let fg_bright_yellow = get_fg("bright_yellow", "\x1b[93m");
    let fg_bright_blue = get_fg("bright_blue", "\x1b[94m");
    let fg_bright_cyan = get_fg("bright_cyan", "\x1b[96m");
    let fg_bright_magenta = get_fg("bright_magenta", "\x1b[95m");
    let fg_bright_white = get_fg("bright_white", "\x1b[97m");

    let num_color = format!("{}{}", fg_bright_white, bold);
    let dot = cfg.colors
        .as_ref()
        .and_then(|c| c.ui.as_ref())
        .and_then(|ui| ui.get("separator"))
        .map(|v| convert_color_value(v))
        .unwrap_or_else(|| format!("{} | {}", fg_gray, reset));

    let use_ascii = data.use_ascii.unwrap_or_else(|| {
        cfg.use_ascii.unwrap_or_else(|| {
            let env_ascii = env::var("USE_ASCII").map(|v| v == "true" || v == "1").unwrap_or(false);
            let mode_ascii = data.mode.as_deref().map(|m| m.eq_ignore_ascii_case("ascii")).unwrap_or(false);
            let raw_ascii = raw_val.get("mode").and_then(|v| v.as_str()).map(|m| m.eq_ignore_ascii_case("ascii")).unwrap_or(false)
                || raw_val.get("use_ascii").and_then(|v| v.as_bool()).unwrap_or(false)
                || raw_val.get("useAscii").and_then(|v| v.as_bool()).unwrap_or(false);
            env_ascii || mode_ascii || raw_ascii
        })
    });

    let use_nerd_fonts = if use_ascii {
        false
    } else {
        data.nerd_fonts_supported.unwrap_or_else(|| {
            env::var("USE_NERD_FONTS").map(|v| v == "true" || v == "1").unwrap_or(false)
        })
    };

    let cfg_icons = cfg.icons.as_ref().and_then(|i| {
        if use_nerd_fonts {
            i.get("nerd_fonts").cloned()
        } else {
            i.get("emoji_fallback").cloned()
        }
    });

    let cat_state = "state";
    let cat_comp = "components";
    let cat_sb = "sandbox";
    let cat_cyc = "cycle";
    let cat_oth = "other";

    let icon_ready = get_icon_str(&cfg_icons, &cfg.icons, cat_state, "ready", if use_ascii { "*" } else if use_nerd_fonts { "\u{f192}" } else { "🟢" });
    let icon_thinking = get_icon_str(&cfg_icons, &cfg.icons, cat_state, "thinking", if use_ascii { "?" } else if use_nerd_fonts { "\u{f07f7}" } else { "💭" });
    let icon_working = get_icon_str(&cfg_icons, &cfg.icons, cat_state, "working", if use_ascii { ">" } else if use_nerd_fonts { "\u{f423}" } else { "⚙" });
    let icon_tool = get_icon_str(&cfg_icons, &cfg.icons, cat_state, "tool", if use_ascii { "=" } else if use_nerd_fonts { "\u{f425}" } else { "⚒" });
    let icon_unknown = get_icon_str(&cfg_icons, &cfg.icons, cat_state, "unknown", if use_ascii { "~" } else if use_nerd_fonts { "\u{f252}" } else { "⏳" });

    let icon_folder = get_icon_str(&cfg_icons, &cfg.icons, cat_comp, "folder", if use_ascii { "DIR:" } else if use_nerd_fonts { "\u{ea83}" } else { "📁" });
    let icon_model = get_icon_str(&cfg_icons, &cfg.icons, cat_comp, "model", if use_ascii { "MOD:" } else if use_nerd_fonts { "\u{f400}" } else { "💡" });
    let icon_branch = get_icon_str(&cfg_icons, &cfg.icons, cat_comp, "branch", if use_ascii { "GIT:" } else if use_nerd_fonts { "\u{f418}" } else { "⎇" });
    let icon_conv = get_icon_str(&cfg_icons, &cfg.icons, cat_comp, "conversation", if use_ascii { "ID:" } else if use_nerd_fonts { "\u{f036a}" } else { "💬" });
    let icon_ctx = get_icon_str(&cfg_icons, &cfg.icons, cat_comp, "context", if use_ascii { "CTX:" } else if use_nerd_fonts { "\u{f134f}" } else { "📊" });
    let icon_tok = get_icon_str(&cfg_icons, &cfg.icons, cat_comp, "token", if use_ascii { "TOK:" } else if use_nerd_fonts { "\u{e26b}" } else { "🪙" });
    let icon_art = get_icon_str(&cfg_icons, &cfg.icons, cat_comp, "artifact", if use_ascii { "ART:" } else if use_nerd_fonts { "\u{f0f6}" } else { "📄" });
    let icon_sub = get_icon_str(&cfg_icons, &cfg.icons, cat_comp, "subagent", if use_ascii { "SUB:" } else if use_nerd_fonts { "\u{f167a}" } else { "🤖" });
    let icon_bg = get_icon_str(&cfg_icons, &cfg.icons, cat_comp, "background_task", if use_ascii { "TASK:" } else if use_nerd_fonts { "\u{f0ae}" } else { "📋" });

    let icon_sb_net = get_icon_str(&cfg_icons, &cfg.icons, cat_sb, "net", if use_ascii { "" } else if use_nerd_fonts { "\u{f0499}" } else { "📦" });
    let icon_sb_nonet = get_icon_str(&cfg_icons, &cfg.icons, cat_sb, "no_net", if use_ascii { "[LOCKED]" } else if use_nerd_fonts { "\u{f0d34}" } else { "📦🔒" });
    let icon_sb_off = get_icon_str(&cfg_icons, &cfg.icons, cat_sb, "off", if use_ascii { "" } else if use_nerd_fonts { "\u{f099c}" } else { "🚫" });

    let icon_cycle_accept = get_icon_str(&cfg_icons, &cfg.icons, cat_cyc, "accept", if use_ascii { "[OK]" } else if use_nerd_fonts { "\u{f012c}" } else { "✅" });
    let icon_cycle_plan = get_icon_str(&cfg_icons, &cfg.icons, cat_cyc, "plan", if use_ascii { "[PLAN]" } else if use_nerd_fonts { "\u{f0349}" } else { "🔍" });

    let icon_yolo = get_icon_str(&cfg_icons, &cfg.icons, cat_oth, "yolo", if use_ascii { "!" } else if use_nerd_fonts { "\u{f06d}" } else { "⚠" });

    let state_raw = data.agent_state.as_deref().unwrap_or("idle");
    let state_seg = match state_raw {
        "idle" => format!("{}{}{} READY{}", fg_bright_green, bold, icon_ready, reset),
        "thinking" => format!("{}{}{} THINKING{}", fg_bright_yellow, bold, icon_thinking, reset),
        "working" => format!("{}{}{} WORKING{}", fg_bright_cyan, bold, icon_working, reset),
        "tool_use" => format!("{}{}{} TOOL{}", fg_bright_magenta, bold, icon_tool, reset),
        _ => format!("{}{}{} {}{}", fg_white, bold, icon_unknown, state_raw.to_uppercase(), reset),
    };

    let model_info = data.model.as_ref();
    let model_id = model_info.and_then(|m| m.id.as_deref()).unwrap_or("");
    let model_name = model_info.and_then(|m| m.display_name.as_deref()).unwrap_or("");
    let effort = model_info.and_then(|m| m.effort.clone().or_else(|| m.effort_level.clone())).unwrap_or_default();

    let base_model = if !model_id.is_empty() { model_id } else { model_name };
    let clean_base = if !base_model.is_empty() {
        base_model.split('(').next().unwrap_or("").trim()
    } else {
        ""
    };
    let model_short = clean_base.replace("gemini-", "").replace("Gemini ", "");

    let model_wide_str = if !effort.is_empty() {
        format!("{} ({})", clean_base, effort)
    } else {
        clean_base.to_string()
    };
    let model_med_str = if !effort.is_empty() {
        format!("{} ({})", model_short, effort)
    } else {
        model_short.clone()
    };

    let m_wide = if !model_wide_str.is_empty() {
        format!("{}{}{} {}{}", fg_bright_magenta, italic, icon_model, model_wide_str, reset)
    } else {
        String::new()
    };
    let m_med = if !model_med_str.is_empty() {
        format!("{}{}{} {}{}", fg_bright_magenta, italic, icon_model, model_med_str, reset)
    } else {
        String::new()
    };
    let m_narrow = if !model_med_str.is_empty() {
        let len = model_med_str.chars().count().min(12);
        format!("{}{}{} {}{}", fg_bright_magenta, italic, icon_model, &model_med_str[..len], reset)
    } else {
        String::new()
    };

    let cwd = data.cwd.as_deref().unwrap_or("");
    let cwd_wide_val = get_shortened_path(cwd, 25, &home_path);
    let dir_wide = if !cwd_wide_val.is_empty() {
        format!("{}{}{} {}{}", fg_cyan, icon_folder, reset, cwd_wide_val, reset)
    } else {
        String::new()
    };

    let cwd_med_val = get_shortened_path(cwd, 15, &home_path);
    let dir_med = if !cwd_med_val.is_empty() {
        format!("{}{}{} {}{}", fg_cyan, icon_folder, reset, cwd_med_val, reset)
    } else {
        String::new()
    };

    let cwd_narrow_val = get_shortened_path(cwd, 0, &home_path);
    let dir_narrow = if !cwd_narrow_val.is_empty() {
        format!("{}{}{} {}{}", fg_cyan, icon_folder, reset, cwd_narrow_val, reset)
    } else {
        String::new()
    };

    let git_branch = get_git_branch(cwd);
    let v_wide = format_branch(git_branch.as_deref(), 15, &fg_blue, &icon_branch, &reset);
    let v_med = format_branch(git_branch.as_deref(), 10, &fg_blue, &icon_branch, &reset);
    let v_narrow = format_branch(git_branch.as_deref(), 6, &fg_blue, &icon_branch, &reset);

    let conv_id = data.conversation_id.as_deref().unwrap_or("");
    let conv_wide = if !conv_id.is_empty() {
        let len = conv_id.chars().count().min(8);
        format!("{}{}{} {}{}", fg_gray, icon_conv, reset, &conv_id[..len], reset)
    } else {
        String::new()
    };
    let conv_med = if !conv_id.is_empty() {
        let len = conv_id.chars().count().min(4);
        format!("{}{}{} {}{}", fg_gray, icon_conv, reset, &conv_id[..len], reset)
    } else {
        String::new()
    };

    let sandbox_default = Sandbox::default();
    let sandbox = data.sandbox.as_ref().unwrap_or(&sandbox_default);
    let sb_wide = format_sandbox(sandbox, "wide", &icon_sb_net, &icon_sb_nonet, &icon_sb_off, &fg_green, &fg_bright_green, &fg_red, &fg_bright_red, &bold, &reset);
    let sb_med = format_sandbox(sandbox, "med", &icon_sb_net, &icon_sb_nonet, &icon_sb_off, &fg_green, &fg_bright_green, &fg_red, &fg_bright_red, &bold, &reset);

    let ctx_default = ContextWindow::default();
    let ctx = data.context_window.as_ref().unwrap_or(&ctx_default);
    let used_pct = ctx.used_percentage.unwrap_or(0.0);
    let pct_int = used_pct as u64;
    let fill_color = if pct_int >= 90 {
        &fg_bright_red
    } else if pct_int >= 60 {
        &fg_bright_yellow
    } else {
        &fg_yellow
    };

    let bar_wide = make_bar(used_pct, 15, fill_color, &fg_gray, &reset, use_ascii);
    let bar_med = make_bar(used_pct, 10, fill_color, &fg_gray, &reset, use_ascii);
    let bar_narrow = make_bar(used_pct, 6, fill_color, &fg_gray, &reset, use_ascii);

    let ctx_bar_wide = format!("{}{}{}  {}{}{:.1}%{}", fg_yellow, icon_ctx, reset, bar_wide, num_color, used_pct, reset);
    let ctx_bar_med = format!("{}{}{}  {}{}{:.1}%{}", fg_yellow, icon_ctx, reset, bar_med, num_color, used_pct, reset);
    let ctx_bar_narrow = format!("{}{}{}  {}{}{}%{}", fg_yellow, icon_ctx, reset, bar_narrow, num_color, pct_int, reset);

    let in_tok = ctx.total_input_tokens.unwrap_or(0);
    let out_tok = ctx.total_output_tokens.unwrap_or(0);
    let limit = ctx.context_window_size.unwrap_or(0);
    let ctx_used = in_tok + out_tok;

    let tok_details_wide = if ctx_used > 0 {
        format!(
            " ({}/{}){}{}{}{}  ({} in/{} out)",
            format_human(ctx_used),
            format_human(limit),
            dot,
            fg_yellow,
            icon_tok,
            reset,
            format_human(in_tok),
            format_human(out_tok)
        )
    } else {
        String::new()
    };

    let tok_details_med = if ctx_used > 0 {
        format!(" ({}/{})", format_human(ctx_used), format_human(limit))
    } else {
        String::new()
    };

    let artifacts = data.artifact_count.unwrap_or(0);
    let bg_tasks = data.task_count.unwrap_or(0);
    let subagents = match &data.subagents {
        Some(serde_json::Value::Number(n)) => n.as_u64().unwrap_or(0),
        Some(serde_json::Value::Array(a)) => a.len() as u64,
        _ => 0,
    };

    let art_wide = format!("{}{}{} {}{}{}", fg_blue, icon_art, reset, num_color, artifacts, reset);
    let sub_wide = format!("{}{}{} {}{}{}", fg_cyan, icon_sub, reset, num_color, subagents, reset);
    let bg_wide = format!("{}{}{} {}{}{}", fg_magenta, icon_bg, reset, num_color, bg_tasks, reset);

    let art_med = format!("{}{}{} {}{}{}", fg_blue, icon_art, reset, num_color, artifacts, reset);
    let sub_med = format!("{}{}{} {}{}{}", fg_cyan, icon_sub, reset, num_color, subagents, reset);
    let bg_med = format!("{}{}{} {}{}{}", fg_magenta, icon_bg, reset, num_color, bg_tasks, reset);

    let art_narrow = format!("{}{}{}{}{}", fg_blue, icon_art, num_color, artifacts, reset);
    let sub_narrow = format!("{}{}{}{}{}", fg_cyan, icon_sub, num_color, subagents, reset);
    let bg_narrow = format!("{}{}{}{}{}", fg_magenta, icon_bg, num_color, bg_tasks, reset);

    let model_disp = if !model_name.is_empty() { model_name } else { model_id };
    let matched_quotas = match_quotas(&data, model_disp);

    let quota_wide = format_quota(&matched_quotas, "wide", &icon_unknown, &fg_cyan, &fg_bright_cyan, &fg_bright_red, &fg_gray, &num_color, &reset, use_ascii);
    let quota_med = format_quota(&matched_quotas, "med", &icon_unknown, &fg_cyan, &fg_bright_cyan, &fg_bright_red, &fg_gray, &num_color, &reset, use_ascii);
    let quota_narrow = format_quota(&matched_quotas, "narrow", &icon_unknown, &fg_cyan, &fg_bright_cyan, &fg_bright_red, &fg_gray, &num_color, &reset, use_ascii);

    let cycle_mode = data.cycle_mode.as_deref().unwrap_or("");
    let cycle_seg = match cycle_mode {
        "accept-edits" => format!("{}{}{} ACCEPT-EDITS{}", fg_bright_yellow, bold, icon_cycle_accept, reset),
        "plan" => format!("{}{}{} PLAN{}", fg_bright_blue, bold, icon_cycle_plan, reset),
        _ => String::new(),
    };

    let cfg_yolo = cfg.show_yolo.unwrap_or(false)
        || cfg.always_show_yolo.unwrap_or(false)
        || cfg.yolo.unwrap_or(false);

    let data_yolo = detect_yolo_in_json(&raw_val)
        || check_process_cmdline_for_yolo()
        || data.yolo.unwrap_or(false)
        || data.is_yolo.unwrap_or(false)
        || data.auto_approve.unwrap_or(false)
        || data.auto_approve_enabled.unwrap_or(false)
        || data.dangerously_skip_permissions.unwrap_or(false)
        || data.skip_permissions.unwrap_or(false)
        || data.approval_mode.as_deref().map(|s| s.eq_ignore_ascii_case("yolo") || s.eq_ignore_ascii_case("auto_approve") || s.eq_ignore_ascii_case("auto-approve")).unwrap_or(false)
        || data.mode.as_deref().map(|s| s.eq_ignore_ascii_case("yolo")).unwrap_or(false);

    let is_yolo_mode = cfg_yolo || data_yolo;

    let yolo_seg = if is_yolo_mode {
        format!("{}{}{} YOLO{}", fg_bright_red, bold, icon_yolo, reset)
    } else {
        String::new()
    };

    let line1_wide = join_with_dot(&[yolo_seg.clone(), state_seg.clone(), cycle_seg.clone(), m_wide.clone(), dir_wide.clone(), v_wide.clone(), conv_wide.clone()], &dot);
    let line2_wide = join_with_dot(&[art_wide.clone(), sub_wide.clone(), bg_wide.clone(), sb_wide.clone(), format!("{}{}", ctx_bar_wide, tok_details_wide), quota_wide.clone()], &dot);

    let _line1_med = join_with_dot(&[yolo_seg.clone(), state_seg.clone(), cycle_seg.clone(), m_med.clone(), dir_med.clone(), v_med.clone()], &dot);
    let _line2_med = join_with_dot(&[art_med.clone(), sub_med.clone(), bg_med.clone(), sb_med.clone(), format!("{}{}", ctx_bar_med, tok_details_med), quota_med.clone()], &dot);

    let cols = data.terminal_width.unwrap_or(80);
    let margin = 8;

    let len1_wide = visible_len(&line1_wide);
    let len2_wide = visible_len(&line2_wide);

    let mut output_lines = Vec::new();

    if cols >= 135 && cols >= (len1_wide + len2_wide + margin) {
        output_lines.push(print_right_aligned(&line1_wide, &line2_wide, cols));
    } else if cols >= 100 {
        let r1_left = join_with_dot(&[yolo_seg.clone(), state_seg.clone(), cycle_seg.clone(), m_wide.clone()], &dot);
        let r1_right = join_with_dot(&[art_wide.clone(), sub_wide.clone(), bg_wide.clone(), sb_wide.clone()], &dot);
        let r2_left = join_with_dot(&[dir_wide.clone(), v_wide.clone(), conv_wide.clone()], &dot);
        let r2_right = join_with_dot(&[format!("{}{}", ctx_bar_wide, tok_details_wide), quota_wide.clone()], &dot);

        output_lines.push(print_right_aligned(&r1_left, &r1_right, cols));
        output_lines.push(print_right_aligned(&r2_left, &r2_right, cols));
    } else if cols >= 75 {
        let r1_left = join_with_dot(&[yolo_seg.clone(), state_seg.clone(), cycle_seg.clone(), m_med.clone()], &dot);
        let r1_right = join_with_dot(&[art_med.clone(), sub_med.clone(), bg_med.clone(), sb_med.clone()], &dot);
        let r2_left = join_with_dot(&[dir_med.clone(), v_med.clone(), conv_med.clone()], &dot);
        let r2_right = join_with_dot(&[format!("{}{}", ctx_bar_med, tok_details_med), quota_med.clone()], &dot);

        output_lines.push(print_right_aligned(&r1_left, &r1_right, cols));
        output_lines.push(print_right_aligned(&r2_left, &r2_right, cols));
    } else if cols >= 50 {
        let r1_left = join_with_dot(&[yolo_seg.clone(), state_seg.clone(), cycle_seg.clone(), m_narrow.clone()], &dot);
        let r1_right = join_with_space(&[art_narrow.clone(), sub_narrow.clone(), bg_narrow.clone()]);
        let r2_left = join_with_dot(&[dir_narrow.clone(), v_narrow.clone()], &dot);
        let r2_right = join_with_dot(&[ctx_bar_narrow.clone(), quota_narrow.clone()], &dot);

        output_lines.push(print_right_aligned(&r1_left, &r1_right, cols));
        output_lines.push(print_right_aligned(&r2_left, &r2_right, cols));
    } else {
        let yolo_short = if !yolo_seg.is_empty() {
            format!("{} ╱ ", yolo_seg)
        } else {
            String::new()
        };
        let cyc_short = if !cycle_seg.is_empty() {
            format!(" {} ╱ {}", fg_gray, cycle_seg)
        } else {
            String::new()
        };
        let m_short = if !model_short.is_empty() {
            let len = model_short.chars().count().min(8);
            format!(" {} ╱ {}{}{}", fg_gray, fg_bright_magenta, &model_short[..len], reset)
        } else {
            String::new()
        };
        output_lines.push(format!("{}{}{}{}", yolo_short, state_seg, cyc_short, m_short));
        output_lines.push(ctx_bar_narrow);
    }

    for line in output_lines {
        println!("{}", line);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_convert_color_value() {
        assert_eq!(convert_color_value(""), "");
        assert_eq!(convert_color_value("#ff0000"), "\x1b[38;2;255;0;0m");
        assert_eq!(convert_color_value("#f00"), "\x1b[38;2;255;0;0m");
        assert_eq!(
            convert_color_value("text #5c6370 text"),
            "text \x1b[38;2;92;99;112m text"
        );
    }

    #[test]
    fn test_strip_ansi_and_visible_len() {
        let colored = "\x1b[38;2;255;0;0mHello\x1b[0m";
        assert_eq!(strip_ansi(colored), "Hello");
        assert_eq!(visible_len(colored), 5);
    }

    #[test]
    fn test_format_human() {
        assert_eq!(format_human(500), "500");
        assert_eq!(format_human(1500), "1.5K");
        assert_eq!(format_human(2_500_000), "2.5M");
    }

    #[test]
    fn test_format_seconds() {
        assert_eq!(format_seconds(0), "0s");
        assert_eq!(format_seconds(45), "45s");
        assert_eq!(format_seconds(120), "2m");
        assert_eq!(format_seconds(3660), "1h1m");
    }

    #[test]
    fn test_get_tokens() {
        assert_eq!(get_tokens("gemini-1.5-pro"), vec!["gemini", "1", "5", "pro"]);
    }

    #[test]
    fn test_match_quotas() {
        let mut quota_map = HashMap::new();
        quota_map.insert(
            "gemini-5h".to_string(),
            QuotaEntry {
                remaining_fraction: Some(0.9),
                reset_in_seconds: Some(16800),
            },
        );
        quota_map.insert(
            "gemini-weekly".to_string(),
            QuotaEntry {
                remaining_fraction: Some(0.74),
                reset_in_seconds: Some(361920),
            },
        );
        let data = InputData {
            quota: Some(quota_map),
            ..Default::default()
        };
        let matched = match_quotas(&data, "gemini-1.5-pro");
        assert!(!matched.is_empty());
        assert_eq!(matched[0].key, "gemini-5h");
    }

    #[test]
    fn test_detect_yolo_in_json() {
        let v1: serde_json::Value = serde_json::from_str(r#"{"sandbox": {"enabled": false}}"#).unwrap();
        assert!(!detect_yolo_in_json(&v1));

        let v2: serde_json::Value = serde_json::from_str(r#"{"flags": {"dangerouslySkipPermissions": true}}"#).unwrap();
        assert!(detect_yolo_in_json(&v2));

        let v3: serde_json::Value = serde_json::from_str(r#"{"config": {"autoApprove": true}}"#).unwrap();
        assert!(detect_yolo_in_json(&v3));

        let v4: serde_json::Value = serde_json::from_str(r#"{"mode": "yolo"}"#).unwrap();
        assert!(detect_yolo_in_json(&v4));
    }


    #[test]
    fn test_make_bar() {
        let bar = make_bar(100.0, 5, "", "", "", false);
        assert_eq!(visible_len(&bar), 5);
        let bar_ascii = make_bar(50.0, 6, "", "", "", true);
        assert_eq!(visible_len(&bar_ascii), 8); // [===   ]
        assert_eq!(bar_ascii, "[===   ]");
    }

    #[test]
    fn test_unicode_width_visible_len() {
        let emoji_str = "🟢 Ready";
        // 🟢 has column width 2, " Ready" has width 6 => total width 8
        assert_eq!(visible_len(emoji_str), 8);
    }

    #[test]
    fn test_get_icon_str_flat_lookup() {
        let active_icons: Option<serde_json::Value> = None;
        let flat_icons: Option<serde_json::Value> = serde_json::from_str(r#"{"ready": "[READY]"}"#).ok();
        let icon = get_icon_str(&active_icons, &flat_icons, "state", "ready", "*");
        assert_eq!(icon, "[READY]");
    }

    #[test]
    fn test_get_shortened_path() {
        assert_eq!(
            get_shortened_path("/home/user/project", 20, "/home/user"),
            "~/project"
        );
    }
}

