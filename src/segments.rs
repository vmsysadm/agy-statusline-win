use std::fs;
use std::path::Path;

use crate::ansi::{format_human, format_seconds, get_tokens};
use crate::data::{InputData, MatchedQuota, Sandbox};
use crate::theme::Theme;

/// Read the current git branch from a working directory.
pub(crate) fn get_git_branch(cwd: &str) -> Option<String> {
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

/// Shorten a path for display, with home-directory replacement.
pub(crate) fn get_shortened_path(path: &str, max_len: usize, home_path: &str) -> String {
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

/// Format a git branch name with icon and color.
pub(crate) fn format_branch(branch: Option<&str>, max_len: usize, fg_blue: &str, icon_branch: &str, reset: &str) -> String {
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

/// Format the sandbox status segment.
pub(crate) fn format_sandbox(
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

/// Build a progress bar string (unicode block or ASCII).
pub(crate) fn make_bar(pct: f64, len: usize, fill_color: &str, fg_gray: &str, reset: &str, use_ascii: bool) -> String {
    let pct_int = pct.clamp(0.0, 100.0);
    let filled = ((pct_int * len as f64) / 100.0).floor() as usize;

    if use_ascii {
        let empty = len.saturating_sub(filled);
        let mut bar = String::with_capacity(len + fill_color.len() + reset.len() + 2);
        bar.push('[');
        bar.push_str(fill_color);
        for _ in 0..filled {
            bar.push('=');
        }
        bar.push_str(reset);
        for _ in 0..empty {
            bar.push(' ');
        }
        bar.push(']');
        bar
    } else {
        let remainder = ((pct_int * len as f64) % 100.0).floor() as usize;

        let block_full = '\u{2588}';
        let block_dark = '\u{2593}';
        let block_med = '\u{2592}';
        let block_light = '\u{2591}';

        let mut bar = String::with_capacity(len * 16);
        for i in 0..len {
            if i < filled {
                bar.push_str(fill_color);
                bar.push(block_full);
                bar.push_str(reset);
            } else if i == filled {
                let partial_block = if remainder >= 75 {
                    block_dark
                } else if remainder >= 50 {
                    block_med
                } else {
                    block_light
                };
                bar.push_str(fill_color);
                bar.push(partial_block);
                bar.push_str(reset);
                bar.push_str(fg_gray);
            } else {
                bar.push_str(fg_gray);
                bar.push(block_light);
                bar.push_str(reset);
            }
        }
        bar
    }
}

/// Format a single quota entry for display.
pub(crate) fn format_single_quota(
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

/// Format all matched quotas for display.
pub(crate) fn format_quota(
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

/// Match quota entries to the current model.
pub(crate) fn match_quotas(data: &InputData, model_disp: &str) -> Vec<MatchedQuota> {
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

/// Build all segment strings for a given density and theme.
pub(crate) struct Segments {
    // State
    pub(crate) state_seg: String,
    // Model — three densities
    pub(crate) m_wide: String,
    pub(crate) m_med: String,
    pub(crate) m_narrow: String,
    // Directory — three densities
    pub(crate) dir_wide: String,
    pub(crate) dir_med: String,
    pub(crate) dir_narrow: String,
    // Branch — three densities
    pub(crate) v_wide: String,
    pub(crate) v_med: String,
    pub(crate) v_narrow: String,
    // Conversation — two densities
    pub(crate) conv_wide: String,
    pub(crate) conv_med: String,
    // Sandbox — three densities
    pub(crate) sb_wide: String,
    pub(crate) sb_med: String,
    pub(crate) sb_narrow: String,
    // Context bar — three densities
    pub(crate) ctx_bar_wide: String,
    pub(crate) ctx_bar_med: String,
    pub(crate) ctx_bar_narrow: String,
    // Token details
    pub(crate) tok_details_wide: String,
    pub(crate) tok_details_med: String,
    // Artifact/subagent/task counters — two densities
    pub(crate) art_wide: String,
    pub(crate) sub_wide: String,
    pub(crate) bg_wide: String,
    pub(crate) art_med: String,
    pub(crate) sub_med: String,
    pub(crate) bg_med: String,
    pub(crate) art_narrow: String,
    pub(crate) sub_narrow: String,
    pub(crate) bg_narrow: String,
    // Quota — three densities
    pub(crate) quota_wide: String,
    pub(crate) quota_med: String,
    pub(crate) quota_narrow: String,
    // Cycle
    pub(crate) cycle_seg: String,
    // YOLO
    pub(crate) yolo_seg: String,
}

impl Segments {
    /// Build all segment strings from input data and theme.
    pub(crate) fn new(data: &InputData, t: &Theme, home_path: &str, yolo_from_json: bool) -> Self {
        let state_raw = data.agent_state.as_deref().unwrap_or("idle");
        let state_seg = match state_raw {
            "idle" => format!("{}{}{} READY{}", t.fg_bright_green, t.bold, t.icon_ready, t.reset),
            "thinking" => format!("{}{}{} THINKING{}", t.fg_bright_yellow, t.bold, t.icon_thinking, t.reset),
            "working" => format!("{}{}{} WORKING{}", t.fg_bright_cyan, t.bold, t.icon_working, t.reset),
            "tool_use" => format!("{}{}{} TOOL{}", t.fg_bright_magenta, t.bold, t.icon_tool, t.reset),
            _ => format!("{}{}{} {}{}", t.fg_white, t.bold, t.icon_unknown, state_raw.to_uppercase(), t.reset),
        };

        // Model
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
            if t.icon_model.is_empty() {
                format!("{}{}{}{}", t.fg_bright_magenta, t.italic, model_wide_str, t.reset)
            } else {
                format!("{}{}{} {}{}", t.fg_bright_magenta, t.italic, t.icon_model, model_wide_str, t.reset)
            }
        } else {
            String::new()
        };
        let m_med = if !model_med_str.is_empty() {
            if t.icon_model.is_empty() {
                format!("{}{}{}{}", t.fg_bright_magenta, t.italic, model_med_str, t.reset)
            } else {
                format!("{}{}{} {}{}", t.fg_bright_magenta, t.italic, t.icon_model, model_med_str, t.reset)
            }
        } else {
            String::new()
        };
        let m_narrow = if !model_med_str.is_empty() {
            let len = model_med_str.chars().count().min(12);
            if t.icon_model.is_empty() {
                format!("{}{}{}{}", t.fg_bright_magenta, t.italic, &model_med_str[..len], t.reset)
            } else {
                format!("{}{}{} {}{}", t.fg_bright_magenta, t.italic, t.icon_model, &model_med_str[..len], t.reset)
            }
        } else {
            String::new()
        };

        // Directory
        let cwd = data.cwd.as_deref().unwrap_or("");
        let cwd_wide_val = get_shortened_path(cwd, 25, home_path);
        let dir_wide = if !cwd_wide_val.is_empty() {
            format!("{}{}{} {}{}", t.fg_cyan, t.icon_folder, t.reset, cwd_wide_val, t.reset)
        } else {
            String::new()
        };

        let cwd_med_val = get_shortened_path(cwd, 15, home_path);
        let dir_med = if !cwd_med_val.is_empty() {
            format!("{}{}{} {}{}", t.fg_cyan, t.icon_folder, t.reset, cwd_med_val, t.reset)
        } else {
            String::new()
        };

        let cwd_narrow_val = get_shortened_path(cwd, 0, home_path);
        let dir_narrow = if !cwd_narrow_val.is_empty() {
            format!("{}{}{} {}{}", t.fg_cyan, t.icon_folder, t.reset, cwd_narrow_val, t.reset)
        } else {
            String::new()
        };

        // Branch
        let git_branch = get_git_branch(cwd);
        let v_wide = format_branch(git_branch.as_deref(), 15, &t.fg_blue, &t.icon_branch, &t.reset);
        let v_med = format_branch(git_branch.as_deref(), 10, &t.fg_blue, &t.icon_branch, &t.reset);
        let v_narrow = format_branch(git_branch.as_deref(), 6, &t.fg_blue, &t.icon_branch, &t.reset);

        // Conversation
        let conv_id = data.conversation_id.as_deref().unwrap_or("");
        let conv_wide = if !conv_id.is_empty() {
            let len = conv_id.chars().count().min(8);
            format!("{}{}{} {}{}", t.fg_gray, t.icon_conv, t.reset, &conv_id[..len], t.reset)
        } else {
            String::new()
        };
        let conv_med = if !conv_id.is_empty() {
            let len = conv_id.chars().count().min(4);
            format!("{}{}{} {}{}", t.fg_gray, t.icon_conv, t.reset, &conv_id[..len], t.reset)
        } else {
            String::new()
        };

        // Sandbox
        let sandbox_default = Sandbox::default();
        let sandbox = data.sandbox.as_ref().unwrap_or(&sandbox_default);
        let sb_wide = format_sandbox(sandbox, "wide", &t.icon_sb_net, &t.icon_sb_nonet, &t.icon_sb_off, &t.fg_green, &t.fg_bright_green, &t.fg_red, &t.fg_bright_red, &t.bold, &t.reset);
        let sb_med = format_sandbox(sandbox, "med", &t.icon_sb_net, &t.icon_sb_nonet, &t.icon_sb_off, &t.fg_green, &t.fg_bright_green, &t.fg_red, &t.fg_bright_red, &t.bold, &t.reset);
        let sb_narrow = format_sandbox(sandbox, "narrow", &t.icon_sb_net, &t.icon_sb_nonet, &t.icon_sb_off, &t.fg_green, &t.fg_bright_green, &t.fg_red, &t.fg_bright_red, &t.bold, &t.reset);

        // Context bar
        let ctx_default = crate::data::ContextWindow::default();
        let ctx = data.context_window.as_ref().unwrap_or(&ctx_default);
        let used_pct = ctx.used_percentage.unwrap_or(0.0);
        let pct_int = used_pct as u64;
        let fill_color = if pct_int >= 90 {
            &t.fg_bright_red
        } else if pct_int >= 60 {
            &t.fg_bright_yellow
        } else {
            &t.fg_yellow
        };

        let bar_wide = make_bar(used_pct, 15, fill_color, &t.fg_gray, &t.reset, t.use_ascii);
        let bar_med = make_bar(used_pct, 10, fill_color, &t.fg_gray, &t.reset, t.use_ascii);
        let bar_narrow = make_bar(used_pct, 6, fill_color, &t.fg_gray, &t.reset, t.use_ascii);

        let ctx_bar_wide = format!("{}{}{} {}{}{:.1}%{}", t.fg_yellow, t.icon_ctx, t.reset, bar_wide, t.num_color, used_pct, t.reset);
        let ctx_bar_med = format!("{}{}{} {}{}{:.1}%{}", t.fg_yellow, t.icon_ctx, t.reset, bar_med, t.num_color, used_pct, t.reset);
        let ctx_bar_narrow = format!("{}{}{} {}{}{}%{}", t.fg_yellow, t.icon_ctx, t.reset, bar_narrow, t.num_color, pct_int, t.reset);

        // Token details
        let in_tok = ctx.total_input_tokens.unwrap_or(0);
        let out_tok = ctx.total_output_tokens.unwrap_or(0);
        let limit = ctx.context_window_size.unwrap_or(0);
        let ctx_used = in_tok + out_tok;

        let tok_details_wide = if ctx_used > 0 {
            format!(
                " ({}/{}){}{}{}{}  ({} in/{} out)",
                format_human(ctx_used),
                format_human(limit),
                t.dot,
                t.fg_yellow,
                t.icon_tok,
                t.reset,
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

        // Artifacts, subagents, background tasks
        let artifacts = data.artifact_count.unwrap_or(0);
        let bg_tasks = data.task_count.unwrap_or(0);
        let subagents = match &data.subagents {
            Some(serde_json::Value::Number(n)) => n.as_u64().unwrap_or(0),
            Some(serde_json::Value::Array(a)) => a.len() as u64,
            _ => 0,
        };

        let art_wide = format!("{}{}{} {}{}{}", t.fg_blue, t.icon_art, t.reset, t.num_color, artifacts, t.reset);
        let sub_wide = format!("{}{}{} {}{}{}", t.fg_cyan, t.icon_sub, t.reset, t.num_color, subagents, t.reset);
        let bg_wide = format!("{}{}{} {}{}{}", t.fg_magenta, t.icon_bg, t.reset, t.num_color, bg_tasks, t.reset);

        let art_med = format!("{}{}{} {}{}{}", t.fg_blue, t.icon_art, t.reset, t.num_color, artifacts, t.reset);
        let sub_med = format!("{}{}{} {}{}{}", t.fg_cyan, t.icon_sub, t.reset, t.num_color, subagents, t.reset);
        let bg_med = format!("{}{}{} {}{}{}", t.fg_magenta, t.icon_bg, t.reset, t.num_color, bg_tasks, t.reset);

        let art_narrow = format!("{}{}{}{}{}", t.fg_blue, t.icon_art, t.num_color, artifacts, t.reset);
        let sub_narrow = format!("{}{}{}{}{}", t.fg_cyan, t.icon_sub, t.num_color, subagents, t.reset);
        let bg_narrow = format!("{}{}{}{}{}", t.fg_magenta, t.icon_bg, t.num_color, bg_tasks, t.reset);

        // Quota
        let model_disp = if !model_name.is_empty() { model_name } else { model_id };
        let matched_quotas = match_quotas(data, model_disp);

        let quota_wide = format_quota(&matched_quotas, "wide", &t.icon_unknown, &t.fg_cyan, &t.fg_bright_cyan, &t.fg_bright_red, &t.fg_gray, &t.num_color, &t.reset, t.use_ascii);
        let quota_med = format_quota(&matched_quotas, "med", &t.icon_unknown, &t.fg_cyan, &t.fg_bright_cyan, &t.fg_bright_red, &t.fg_gray, &t.num_color, &t.reset, t.use_ascii);
        let quota_narrow = format_quota(&matched_quotas, "narrow", &t.icon_unknown, &t.fg_cyan, &t.fg_bright_cyan, &t.fg_bright_red, &t.fg_gray, &t.num_color, &t.reset, t.use_ascii);

        // Cycle mode
        let cycle_mode = data.cycle_mode.as_deref().unwrap_or("");
        let cycle_seg = match cycle_mode {
            "accept-edits" => format!("{}{}{} ACCEPT-EDITS{}", t.fg_bright_yellow, t.bold, t.icon_cycle_accept, t.reset),
            "plan" => format!("{}{}{} PLAN{}", t.fg_bright_blue, t.bold, t.icon_cycle_plan, t.reset),
            _ => String::new(),
        };

        // YOLO
        // We need cfg for the config-level yolo flags.
        // Since Segments::new doesn't receive cfg directly, we compute yolo from the inputs.
        // The caller passes yolo_from_json which already includes JSON + process check.
        let data_yolo = yolo_from_json
            || data.yolo.unwrap_or(false)
            || data.is_yolo.unwrap_or(false)
            || data.auto_approve.unwrap_or(false)
            || data.auto_approve_enabled.unwrap_or(false)
            || data.dangerously_skip_permissions.unwrap_or(false)
            || data.skip_permissions.unwrap_or(false)
            || data.approval_mode.as_deref().map(|s| s.eq_ignore_ascii_case("yolo") || s.eq_ignore_ascii_case("auto_approve") || s.eq_ignore_ascii_case("auto-approve")).unwrap_or(false)
            || data.mode.as_deref().map(|s| s.eq_ignore_ascii_case("yolo")).unwrap_or(false);

        let yolo_seg = if data_yolo {
            format!("{}{}{} YOLO{}", t.fg_bright_red, t.bold, t.icon_yolo, t.reset)
        } else {
            String::new()
        };

        Segments {
            state_seg,
            m_wide, m_med, m_narrow,
            dir_wide, dir_med, dir_narrow,
            v_wide, v_med, v_narrow,
            conv_wide, conv_med,
            sb_wide, sb_med, sb_narrow,
            ctx_bar_wide, ctx_bar_med, ctx_bar_narrow,
            tok_details_wide, tok_details_med,
            art_wide, sub_wide, bg_wide,
            art_med, sub_med, bg_med,
            art_narrow, sub_narrow, bg_narrow,
            quota_wide, quota_med, quota_narrow,
            cycle_seg,
            yolo_seg,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use crate::ansi::visible_len;
    use crate::data::QuotaEntry;

    #[test]
    fn test_make_bar() {
        let bar = make_bar(100.0, 5, "", "", "", false);
        assert_eq!(visible_len(&bar), 5);
        let bar_ascii = make_bar(50.0, 6, "", "", "", true);
        assert_eq!(visible_len(&bar_ascii), 8); // [===   ]
        assert_eq!(bar_ascii, "[===   ]");
    }

    #[test]
    fn test_get_shortened_path() {
        assert_eq!(
            get_shortened_path("/home/user/project", 20, "/home/user"),
            "~/project"
        );
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
}
