use serde::Deserialize;
use std::collections::HashMap;
use std::env;
use std::fs;
use std::io::{self, Read};
use std::path::Path;

#[derive(Deserialize, Debug, Default)]
struct QuotaEntry {
    #[serde(default)]
    remaining_fraction: Option<f64>,
    #[serde(default)]
    reset_in_seconds: Option<u64>,
}

#[derive(Deserialize, Debug, Default)]
struct ContextWindow {
    #[serde(default)]
    used_percentage: Option<f64>,
    #[serde(default)]
    total_input_tokens: Option<u64>,
    #[serde(default)]
    total_output_tokens: Option<u64>,
    #[serde(default)]
    context_window_size: Option<u64>,
}

#[derive(Deserialize, Debug, Default)]
struct Sandbox {
    #[serde(default)]
    enabled: Option<bool>,
    #[serde(default)]
    allow_network: Option<bool>,
}

#[derive(Deserialize, Debug, Default)]
struct Model {
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    display_name: Option<String>,
    #[serde(default)]
    effort: Option<String>,
    #[serde(default)]
    effort_level: Option<String>,
}

#[derive(Deserialize, Debug, Default)]
struct InputData {
    #[serde(default)]
    agent_state: Option<String>,
    #[serde(default)]
    context_window: Option<ContextWindow>,
    #[serde(default)]
    sandbox: Option<Sandbox>,
    #[serde(default)]
    artifact_count: Option<u64>,
    #[serde(default)]
    task_count: Option<u64>,
    #[serde(default)]
    subagents: Option<serde_json::Value>,
    #[serde(default)]
    model: Option<Model>,
    #[serde(default)]
    terminal_width: Option<usize>,
    #[serde(default)]
    cwd: Option<String>,
    #[serde(default)]
    conversation_id: Option<String>,
    #[serde(default)]
    cycle_mode: Option<String>,
    #[serde(default)]
    quota: Option<HashMap<String, QuotaEntry>>,
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
    strip_ansi(s).chars().count()
}

fn format_human(n: u64) -> String {
    if n >= 1_000_000 {
        format!("{:.1}M", n as f64 / 1_000_000.0)
    } else if n >= 1_000 {
        format!("{:.1}K", n as f64 / 1_000.0)
    } else {
        n.to_string()
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

fn shorten_path(path: &str, max_len: usize) -> String {
    if path.is_empty() {
        return String::new();
    }
    let home = env::var("USERPROFILE")
        .or_else(|_| env::var("HOME"))
        .unwrap_or_default();
    let mut p = path.to_string();
    if !home.is_empty() && p.starts_with(&home) {
        p = format!("~{}", &p[home.len()..]);
    }
    if max_len == 0 {
        if p == "~" {
            p
        } else {
            Path::new(path)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or(&p)
                .to_string()
        }
    } else if p.len() > max_len {
        let leaf = Path::new(path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(&p);
        format!("...{}", leaf)
    } else {
        p
    }
}

fn make_bar(pct: f64, len: usize, fill_color: &str, fg_gray: &str, reset: &str) -> String {
    let pct_int = (pct.clamp(0.0, 100.0)) as usize;
    let filled = (pct_int * len) / 100;
    let remainder = (pct_int * len) % 100;

    let block_full = "\u{2588}";
    let block_dark = "\u{2593}";
    let block_med = "\u{2592}";
    let block_light = "\u{2591}";

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

fn print_right_aligned(left: &str, right: &str, total_cols: usize) -> String {
    let left_vis = visible_len(left);
    let right_vis = visible_len(right);

    if left.is_empty() {
        return right.to_string();
    }
    if right.is_empty() {
        return left.to_string();
    }

    if total_cols > left_vis + right_vis {
        let pad = total_cols - left_vis - right_vis;
        format!("{}{}{}", left, " ".repeat(pad), right)
    } else {
        format!("{}  {}", left, right)
    }
}

fn join_with_dot(parts: &[&str], dot: &str) -> String {
    parts
        .iter()
        .filter(|s| !s.is_empty())
        .copied()
        .collect::<Vec<&str>>()
        .join(dot)
}

fn main() {
    let mut input = String::new();
    let _ = io::stdin().read_to_string(&mut input);

    let data: InputData = serde_json::from_str(&input).unwrap_or_default();

    let state = data.agent_state.as_deref().unwrap_or("idle");
    let cwd = data.cwd.as_deref().unwrap_or("");
    let cols = data.terminal_width.unwrap_or(80);

    let reset = "\x1b[0m";
    let bold = "\x1b[1m";
    let italic = "\x1b[3m";

    let fg_blue = "\x1b[34m";
    let fg_cyan = "\x1b[36m";
    let fg_green = "\x1b[32m";
    let fg_red = "\x1b[31m";
    let fg_yellow = "\x1b[33m";
    let fg_white = "\x1b[37m";
    let fg_gray = "\x1b[90m";

    let fg_bright_red = "\x1b[91m";
    let fg_bright_green = "\x1b[92m";
    let fg_bright_yellow = "\x1b[93m";
    let fg_bright_blue = "\x1b[94m";
    let fg_bright_magenta = "\x1b[95m";
    let fg_bright_cyan = "\x1b[96m";
    let fg_bright_white = "\x1b[97m";

    let dot = format!("{} | {}", fg_gray, reset);

    // 1. Agent State
    let state_seg = match state {
        "idle" => format!("{}{}\u{f192} READY{}", fg_bright_green, bold, reset),
        "thinking" => format!("{}{}\u{f07f7} THINKING{}", fg_bright_yellow, bold, reset),
        "working" => format!("{}{}\u{f423} WORKING{}", fg_bright_cyan, bold, reset),
        "tool_use" => format!("{}{}\u{f425} TOOL{}", fg_bright_magenta, bold, reset),
        _ => format!("{}{}\u{f252} {}{}", fg_white, bold, state.to_uppercase(), reset),
    };

    // 2. Cycle Mode
    let cycle_mode = data.cycle_mode.as_deref().unwrap_or("");
    let cycle_seg = match cycle_mode {
        "accept-edits" => format!("{}{}\u{f012c} ACCEPT-EDITS{}", fg_bright_yellow, bold, reset),
        "plan" => format!("{}{}\u{f0349} PLAN{}", fg_bright_blue, bold, reset),
        _ => String::new(),
    };

    // 3. Model
    let model_info = data.model.unwrap_or_default();
    let model_id = model_info.id.as_deref().unwrap_or("");
    let model_name = model_info.display_name.as_deref().unwrap_or("");
    let effort = model_info
        .effort
        .or(model_info.effort_level)
        .unwrap_or_default();

    let base_model = if !model_name.is_empty() {
        model_name
    } else {
        model_id
    };
    let clean_model = base_model.replace("gemini-", "").replace("Gemini ", "");

    let model_str = if !effort.is_empty() {
        format!("{} ({})", clean_model, effort)
    } else {
        clean_model.clone()
    };

    let model_seg_wide = if !model_str.is_empty() {
        format!("{}{}\u{f400} {}{}", fg_bright_magenta, italic, model_str, reset)
    } else {
        String::new()
    };
    let model_seg_med = if !model_str.is_empty() {
        format!("{}{}\u{f400} {}{}", fg_bright_magenta, italic, clean_model, reset)
    } else {
        String::new()
    };

    // 4. Directory CWD
    let dir_wide_val = shorten_path(cwd, 25);
    let dir_wide = if !dir_wide_val.is_empty() {
        format!("{}\u{ea83} {}{}", fg_cyan, dir_wide_val, reset)
    } else {
        String::new()
    };
    let dir_med_val = shorten_path(cwd, 15);
    let dir_med = if !dir_med_val.is_empty() {
        format!("{}\u{ea83} {}{}", fg_cyan, dir_med_val, reset)
    } else {
        String::new()
    };

    // 5. Git Branch
    let branch = get_git_branch(cwd);
    let branch_seg_wide = if let Some(ref b) = branch {
        let b_short = if b.len() > 15 { &b[..15] } else { b };
        format!("{}\u{f418} {}{}", fg_bright_blue, b_short, reset)
    } else {
        String::new()
    };
    let branch_seg_med = if let Some(ref b) = branch {
        let b_short = if b.len() > 10 { &b[..10] } else { b };
        format!("{}\u{f418} {}{}", fg_bright_blue, b_short, reset)
    } else {
        String::new()
    };

    // 6. Conversation ID
    let conv_id = data.conversation_id.as_deref().unwrap_or("");
    let conv_seg = if !conv_id.is_empty() {
        let cid = if conv_id.len() > 8 { &conv_id[..8] } else { conv_id };
        format!("{}\u{f036a} {}{}", fg_gray, cid, reset)
    } else {
        String::new()
    };

    // 7. Artifacts, Subagents, Tasks
    let artifacts = data.artifact_count.unwrap_or(0);
    let tasks = data.task_count.unwrap_or(0);
    let subagents = match data.subagents {
        Some(serde_json::Value::Array(ref arr)) => arr.len() as u64,
        Some(serde_json::Value::Number(ref n)) => n.as_u64().unwrap_or(0),
        _ => 0,
    };

    let art_seg = format!("{}\u{f0f6} {}{}{}{}", fg_blue, fg_bright_white, bold, artifacts, reset);
    let sub_seg = format!("{}\u{f167a} {}{}{}{}", fg_cyan, fg_bright_white, bold, subagents, reset);
    let bg_seg = format!("{}\u{f0ae} {}{}{}{}", fg_bright_magenta, fg_bright_white, bold, tasks, reset);

    // 8. Sandbox
    let sandbox_info = data.sandbox.unwrap_or_default();
    let sb_enabled = sandbox_info.enabled.unwrap_or(false);
    let sb_net = sandbox_info.allow_network.unwrap_or(false);
    let sb_seg = if sb_enabled {
        if sb_net {
            format!("{}\u{f0499} {}{}ON (net){}", fg_green, fg_bright_green, bold, reset)
        } else {
            format!("{}\u{f0d34} {}{}ON (no-net){}", fg_green, fg_bright_green, bold, reset)
        }
    } else {
        format!("{}\u{f099c} {}{}OFF{}", fg_red, fg_bright_red, bold, reset)
    };

    // 9. Context Window & Bar
    let ctx = data.context_window.unwrap_or_default();
    let used_pct = ctx.used_percentage.unwrap_or(0.0);
    let in_tok = ctx.total_input_tokens.unwrap_or(0);
    let out_tok = ctx.total_output_tokens.unwrap_or(0);
    let total_used = in_tok + out_tok;
    let limit = ctx.context_window_size.unwrap_or(0);

    let fill_color = if used_pct >= 90.0 {
        fg_bright_red
    } else if used_pct >= 60.0 {
        fg_bright_yellow
    } else {
        fg_yellow
    };

    let bar_wide = make_bar(used_pct, 15, fill_color, fg_gray, reset);
    let bar_med = make_bar(used_pct, 10, fill_color, fg_gray, reset);

    let tok_detail_wide = if total_used > 0 && limit > 0 {
        format!(
            " ({}/{}){} {}\u{e26b} ({} in/{} out)",
            format_human(total_used),
            format_human(limit),
            dot,
            fg_yellow,
            format_human(in_tok),
            format_human(out_tok)
        )
    } else {
        String::new()
    };

    let tok_detail_med = if total_used > 0 && limit > 0 {
        format!(" ({}/{})", format_human(total_used), format_human(limit))
    } else {
        String::new()
    };

    let ctx_seg_wide = format!(
        "{}\u{f134f} {}{}{}{:.1}%{}{}",
        fg_yellow, bar_wide, fg_bright_white, bold, used_pct, reset, tok_detail_wide
    );

    let ctx_seg_med = format!(
        "{}\u{f134f} {}{}{}{:.1}%{}{}",
        fg_yellow, bar_med, fg_bright_white, bold, used_pct, reset, tok_detail_med
    );

    // 10. Quota
    let mut quota_parts = Vec::new();
    if let Some(ref map) = data.quota {
        let mut sorted: Vec<(&String, &QuotaEntry)> = map.iter().collect();
        sorted.sort_by_key(|(k, _)| (*k).clone());
        for (key, q) in sorted {
            if let Some(frac) = q.remaining_fraction {
                let pct = (frac * 100.0).round() as u64;
                let reset_s = q.reset_in_seconds.unwrap_or(0);
                let clean_name = if key.contains("5h") || key.contains("five") {
                    "5h"
                } else if key.contains("weekly") {
                    "wk"
                } else {
                    key.as_str()
                };
                quota_parts.push(format!(
                    "{}{}{}%{} {}{}{} {}{}{}",
                    fg_bright_white,
                    bold,
                    pct,
                    reset,
                    fg_gray,
                    clean_name,
                    reset,
                    fg_gray,
                    format_seconds(reset_s),
                    reset
                ));
            }
        }
    }
    let quota_seg = quota_parts.join(" ");

    // Build Lines based on terminal width `cols`
    if cols >= 100 {
        let r1_left = join_with_dot(&[&state_seg, &cycle_seg, &model_seg_wide], &dot);
        let r1_right = join_with_dot(&[&art_seg, &sub_seg, &bg_seg, &sb_seg], &dot);
        let r2_left = join_with_dot(&[&dir_wide, &branch_seg_wide, &conv_seg], &dot);
        let r2_right = join_with_dot(&[&ctx_seg_wide, &quota_seg], &dot);

        println!("{}", print_right_aligned(&r1_left, &r1_right, cols));
        println!("{}", print_right_aligned(&r2_left, &r2_right, cols));
    } else if cols >= 75 {
        let r1_left = join_with_dot(&[&state_seg, &cycle_seg, &model_seg_med], &dot);
        let r1_right = join_with_dot(&[&art_seg, &sub_seg, &bg_seg, &sb_seg], &dot);
        let r2_left = join_with_dot(&[&dir_med, &branch_seg_med], &dot);
        let r2_right = join_with_dot(&[&ctx_seg_med, &quota_seg], &dot);

        println!("{}", print_right_aligned(&r1_left, &r1_right, cols));
        println!("{}", print_right_aligned(&r2_left, &r2_right, cols));
    } else {
        let l1 = join_with_dot(&[&state_seg, &cycle_seg, &model_seg_med, &dir_med], &dot);
        let l2 = join_with_dot(&[&ctx_seg_med, &quota_seg], &dot);

        println!("{}", l1);
        println!("{}", l2);
    }
}
