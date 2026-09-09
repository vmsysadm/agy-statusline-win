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
            b_name = format!("{}..", b_name.chars().take(max_len).collect::<String>());
        }
        format!("{}{}{} {}{}", fg_blue, icon_branch, reset, b_name, reset)
    } else {
        String::new()
    }
}

/// Format the sandbox status segment.
pub(crate) fn format_sandbox(sandbox: &Sandbox, mode: &str, t: &Theme) -> String {
    let enabled = sandbox.enabled.unwrap_or(false);
    let net = sandbox.allow_network.unwrap_or(false);

    if enabled {
        let icon = if net { &t.icon_sb_net } else { &t.icon_sb_nonet };
        if mode == "wide" {
            let label = if net { "ON (net)" } else { "ON (no-net)" };
            format!("{}{}{} {}{}{}{}", t.fg_green, icon, t.reset, t.fg_bright_green, t.bold, label, t.reset)
        } else if mode == "med" {
            format!("{}{}{} {}{}ON{}", t.fg_green, icon, t.reset, t.fg_bright_green, t.bold, t.reset)
        } else {
            format!("{}{}{}", t.fg_green, icon, t.reset)
        }
    } else if mode == "wide" || mode == "med" {
        format!("{}{}{} {}{}OFF{}", t.fg_red, t.icon_sb_off, t.reset, t.fg_bright_red, t.bold, t.reset)
    } else {
        format!("{}{}{}", t.fg_red, t.icon_sb_off, t.reset)
    }
}

/// Build a progress bar string (unicode block or ASCII).
pub(crate) fn make_bar(pct: f64, len: usize, fill_color: &str, fg_gray: &str, reset: &str, use_ascii: bool) -> String {
    let pct_int = pct.clamp(0.0, 100.0);
    let mut filled = ((pct_int * len as f64) / 100.0).round() as usize;
    if pct_int > 0.0 && filled == 0 {
        filled = 1;
    }

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
pub(crate) fn format_single_quota(entry: &MatchedQuota, mode: &str, t: &Theme) -> String {
    let (fg_cyan, fg_gray, num_color, reset) = (&t.fg_cyan, &t.fg_gray, &t.num_color, &t.reset);
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
        &t.fg_bright_red
    } else if pct <= 40 {
        &t.fg_bright_cyan
    } else {
        fg_cyan
    };

    if mode == "narrow" {
        return format!("{}{}%{} {}{}{}", num_color, pct, reset, fg_gray, clean_name, reset);
    }

    if mode == "med" {
        return format!(
            "{}{}%{} {}{}{} {}{}{}",
            num_color, pct, reset, fg_gray, clean_name, reset, fg_gray, q_reset, reset
        );
    }

    let bar = make_bar(pct as f64, 5, q_color, fg_gray, reset, t.use_ascii);
    format!(
        "{}{}{}%{} {}{}{} {}{}{}",
        bar, num_color, pct, reset, fg_gray, clean_name, reset, fg_gray, q_reset, reset
    )
}

/// Format all matched quotas for display.
pub(crate) fn format_quota(matched_quotas: &[MatchedQuota], mode: &str, t: &Theme) -> String {
    if matched_quotas.is_empty() {
        return String::new();
    }

    if mode == "narrow" {
        return format_single_quota(&matched_quotas[0], mode, t);
    }

    matched_quotas
        .iter()
        .map(|e| format_single_quota(e, mode, t))
        .collect::<Vec<_>>()
        .join(" ")
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
            "idle" | "ready" | "review" | "reviewing" => format!("{}{}{} READY{}", t.fg_bright_green, t.bold, t.icon_ready, t.reset),
            "waiting_for_input" | "waiting" => format!("{}{}{} WAITING{}", t.fg_bright_yellow, t.bold, t.icon_waiting, t.reset),
            "thinking" => format!("{}{}{} THINKING{}", t.fg_bright_yellow, t.bold, t.icon_thinking, t.reset),
            "working" => format!("{}{}{} WORKING{}", t.fg_bright_cyan, t.bold, t.icon_working, t.reset),
            "tool_use" | "tool" => format!("{}{}{} TOOL{}", t.fg_bright_magenta, t.bold, t.icon_tool, t.reset),
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

        let model_seg = |label: &str| -> String {
            if label.is_empty() {
                String::new()
            } else if t.icon_model.is_empty() {
                format!("{}{}{}{}", t.fg_bright_magenta, t.italic, label, t.reset)
            } else {
                format!("{}{}{} {}{}", t.fg_bright_magenta, t.italic, t.icon_model, label, t.reset)
            }
        };

        let m_wide = model_seg(&model_wide_str);
        let m_med = model_seg(&model_med_str);
        let m_narrow = model_seg(&model_med_str.chars().take(12).collect::<String>());

        // Directory
        let cwd = data.cwd.as_deref().unwrap_or("");
        let dir_seg = |max_len: usize| -> String {
            let val = get_shortened_path(cwd, max_len, home_path);
            if val.is_empty() {
                String::new()
            } else {
                format!("{}{}{} {}{}", t.fg_cyan, t.icon_folder, t.reset, val, t.reset)
            }
        };
        let dir_wide = dir_seg(25);
        let dir_med = dir_seg(15);
        let dir_narrow = dir_seg(0);

        // Branch
        let git_branch = get_git_branch(cwd);
        let v_wide = format_branch(git_branch.as_deref(), 15, &t.fg_blue, &t.icon_branch, &t.reset);
        let v_med = format_branch(git_branch.as_deref(), 10, &t.fg_blue, &t.icon_branch, &t.reset);
        let v_narrow = format_branch(git_branch.as_deref(), 6, &t.fg_blue, &t.icon_branch, &t.reset);

        // Conversation
        let conv_id = data.conversation_id.as_deref().unwrap_or("");
        let conv_seg = |len: usize| -> String {
            if conv_id.is_empty() {
                String::new()
            } else {
                let short: String = conv_id.chars().take(len).collect();
                format!("{}{}{} {}{}", t.fg_gray, t.icon_conv, t.reset, short, t.reset)
            }
        };
        let conv_wide = conv_seg(8);
        let conv_med = conv_seg(4);

        // Sandbox
        let sandbox_default = Sandbox::default();
        let sandbox = data.sandbox.as_ref().unwrap_or(&sandbox_default);
        let sb_wide = format_sandbox(sandbox, "wide", t);
        let sb_med = format_sandbox(sandbox, "med", t);
        let sb_narrow = format_sandbox(sandbox, "narrow", t);

        // Context bar
        let ctx_default = crate::data::ContextWindow::default();
        let ctx = data.context_window.as_ref().unwrap_or(&ctx_default);
        let in_tok = ctx.total_input_tokens.unwrap_or(0);
        let out_tok = ctx.total_output_tokens.unwrap_or(0);
        let limit = ctx.context_window_size.unwrap_or(0);
        let used_pct = ctx.used_percentage.unwrap_or_else(|| {
            if limit > 0 {
                (in_tok as f64 / limit as f64) * 100.0
            } else {
                0.0
            }
        });
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

        let ctx_used = in_tok;
        let ctx_fraction = if ctx_used > 0 || limit > 0 {
            format!(" ({}/{})", format_human(ctx_used), format_human(limit))
        } else {
            String::new()
        };

        let ctx_bar_wide = format!("{}{}{} {}{}{:.1}%{}{}", t.fg_yellow, t.icon_ctx, t.reset, bar_wide, t.num_color, used_pct, t.reset, ctx_fraction);
        let ctx_bar_med = format!("{}{}{} {}{}{:.1}%{}{}", t.fg_yellow, t.icon_ctx, t.reset, bar_med, t.num_color, used_pct, t.reset, ctx_fraction);
        let ctx_bar_narrow = format!("{}{}{} {}{}{}%{}", t.fg_yellow, t.icon_ctx, t.reset, bar_narrow, t.num_color, pct_int, t.reset);

        // Token details
        let (in_display, out_display, cache_display) = if let Some(ref cur) = ctx.current_usage {
            let cur_in = cur.input_tokens.unwrap_or(0);
            let cur_out = cur.output_tokens.unwrap_or(0);
            let cur_cache = cur.cache_read_input_tokens.unwrap_or(0);
            (cur_in, cur_out, cur_cache)
        } else {
            (in_tok, out_tok, 0)
        };

        let tok_breakdown_wide = if cache_display > 0 {
            format!(
                "({} in/{} out/{} cache)",
                format_human(in_display),
                format_human(out_display),
                format_human(cache_display)
            )
        } else {
            format!(
                "({} in/{} out)",
                format_human(in_display),
                format_human(out_display)
            )
        };

        let tok_breakdown_med = format!(
            "({} in/{} out)",
            format_human(in_display),
            format_human(out_display)
        );

        let tok_details_wide = if in_display > 0 || out_display > 0 || cache_display > 0 {
            format!(
                "{}{}{} {}",
                t.fg_yellow,
                t.icon_tok,
                t.reset,
                tok_breakdown_wide
            )
        } else {
            String::new()
        };

        let tok_details_med = if in_display > 0 || out_display > 0 {
            format!(
                "{}{}{} {}",
                t.fg_yellow,
                t.icon_tok,
                t.reset,
                tok_breakdown_med
            )
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

        let art_narrow = format!("{}{}{}{}{}", t.fg_blue, t.icon_art, t.num_color, artifacts, t.reset);
        let sub_narrow = format!("{}{}{}{}{}", t.fg_cyan, t.icon_sub, t.num_color, subagents, t.reset);
        let bg_narrow = format!("{}{}{}{}{}", t.fg_magenta, t.icon_bg, t.num_color, bg_tasks, t.reset);

        // Quota
        let model_disp = if !model_name.is_empty() { model_name } else { model_id };
        let matched_quotas = match_quotas(data, model_disp);

        let quota_wide = format_quota(&matched_quotas, "wide", t);
        let quota_med = format_quota(&matched_quotas, "med", t);
        let quota_narrow = format_quota(&matched_quotas, "narrow", t);

        // Cycle mode
        let cycle_mode = data.cycle_mode.as_deref().unwrap_or("");
        let cycle_seg = match cycle_mode {
            "accept-edits" => format!("{}{}{} ACCEPT-EDITS{}", t.fg_bright_yellow, t.bold, t.icon_cycle_accept, t.reset),
            "plan" => format!("{}{}{} PLAN{}", t.fg_bright_blue, t.bold, t.icon_cycle_plan, t.reset),
            _ => String::new(),
        };

        // YOLO — the caller already folded together the recursive JSON scan, the parent
        // process check and the config flag.
        let yolo_seg = if yolo_from_json {
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
        let bar_ascii_low = make_bar(4.2, 15, "", "", "", true);
        assert_eq!(bar_ascii_low, "[=              ]");
    }

    /// A stand-in home directory, built from the platform's own temp location so the test carries
    /// no drive letter, separator, or assumption about where homes live.
    fn fake_home() -> std::path::PathBuf {
        std::env::temp_dir().join("agy-home")
    }

    #[test]
    fn test_get_shortened_path() {
        let home = fake_home();
        let cwd = home.join("project");
        let sep = std::path::MAIN_SEPARATOR;

        // The home prefix collapses to `~`, and the separator is whatever the platform uses --
        // the previous fixture was `/home/user`, a shape this Windows-only tool never receives,
        // so the `\`-separated case it actually ships against went unasserted.
        assert_eq!(
            get_shortened_path(&cwd.to_string_lossy(), 20, &home.to_string_lossy()),
            format!("~{}project", sep)
        );

        // A path outside the home directory keeps its own shape.
        let outside = std::env::temp_dir().join("elsewhere").join("project");
        assert_eq!(
            get_shortened_path(&outside.to_string_lossy(), 0, &home.to_string_lossy()),
            "project"
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

    #[test]
    fn test_context_window_calculation() {
        let data = InputData {
            context_window: Some(crate::data::ContextWindow {
                total_input_tokens: Some(126936),
                total_output_tokens: Some(62398),
                context_window_size: Some(250000),
                used_percentage: Some(50.7744),
                ..Default::default()
            }),
            ..Default::default()
        };
        let cfg = crate::config::Config::default();
        let theme = Theme::new(&data, &cfg);
        let segs = Segments::new(&data, &theme, &fake_home().to_string_lossy(), false);
        assert!(segs.ctx_bar_wide.contains("50.8%"));
        assert!(segs.ctx_bar_wide.contains("126.9K/250.0K") || segs.ctx_bar_wide.contains("127.0K/250.0K") || segs.ctx_bar_wide.contains("126.9K") || segs.ctx_bar_wide.contains("127K"));
        assert!(!segs.ctx_bar_wide.contains("189.3K/250.0K"));
    }

    #[test]
    fn test_context_window_with_current_usage() {
        let data = InputData {
            context_window: Some(crate::data::ContextWindow {
                total_input_tokens: Some(128164),
                total_output_tokens: Some(46300),
                context_window_size: Some(1048576),
                used_percentage: Some(12.2226),
                current_usage: Some(crate::data::CurrentUsage {
                    input_tokens: Some(5585),
                    output_tokens: Some(120),
                    cache_read_input_tokens: Some(131771),
                }),
            }),
            ..Default::default()
        };
        let cfg = crate::config::Config::default();
        let theme = Theme::new(&data, &cfg);
        let segs = Segments::new(&data, &theme, &fake_home().to_string_lossy(), false);
        assert!(segs.ctx_bar_wide.contains("12.2%"));
        assert!(segs.ctx_bar_wide.contains("128.2K/1.0M"));
        assert!(segs.tok_details_wide.contains("5.6K in"));
        assert!(segs.tok_details_wide.contains("120 out"));
        assert!(segs.tok_details_wide.contains("131.8K cache"));
    }

    #[test]
    fn test_state_normalization() {
        // review -> READY
        let data_review = InputData {
            agent_state: Some("review".to_string()),
            ..Default::default()
        };
        let mut cfg_ascii = crate::config::Config::default();
        cfg_ascii.use_ascii = Some(true);
        let theme_ascii = Theme::new(&data_review, &cfg_ascii);
        let segs_review = Segments::new(&data_review, &theme_ascii, &fake_home().to_string_lossy(), false);
        assert!(segs_review.state_seg.contains("READY"));
        assert!(segs_review.state_seg.contains("*"));

        // waiting_for_input -> WAITING
        let data_waiting = InputData {
            agent_state: Some("waiting_for_input".to_string()),
            ..Default::default()
        };
        let segs_waiting = Segments::new(&data_waiting, &theme_ascii, &fake_home().to_string_lossy(), false);
        assert!(segs_waiting.state_seg.contains("WAITING"));
        assert!(segs_waiting.state_seg.contains("~"));

        // waiting -> WAITING
        let data_waiting_alias = InputData {
            agent_state: Some("waiting".to_string()),
            ..Default::default()
        };
        let segs_waiting_alias = Segments::new(&data_waiting_alias, &theme_ascii, &fake_home().to_string_lossy(), false);
        assert!(segs_waiting_alias.state_seg.contains("WAITING"));

        // idle -> READY
        let data_idle = InputData {
            agent_state: Some("idle".to_string()),
            ..Default::default()
        };
        let segs_idle = Segments::new(&data_idle, &theme_ascii, &fake_home().to_string_lossy(), false);
        assert!(segs_idle.state_seg.contains("READY"));
    }
}
