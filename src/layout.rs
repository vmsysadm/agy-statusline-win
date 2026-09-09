use crate::ansi::{truncate_to_visible_width, visible_len};
use crate::config::Config;
use crate::data::InputData;
use crate::segments::Segments;
use crate::theme::Theme;
use crate::yolo;

/// Join non-empty items with a separator.
fn join_non_empty(items: &[&str], sep: &str) -> String {
    items.iter().filter(|s| !s.is_empty()).copied().collect::<Vec<_>>().join(sep)
}


/// Right-align two segments within a given column width.
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

fn pad_to_visible_len(s: &str, target_len: usize) -> String {
    format!("{}{}", s, " ".repeat(target_len.saturating_sub(visible_len(s))))
}

/// Build the final statusline output (1 or 2 lines).
pub(crate) fn render_statusline(
    data: &InputData,
    cfg: &Config,
    t: &Theme,
    yolo_from_json: bool,
    home_path: &str,
) -> Vec<String> {
    let full_yolo = yolo_from_json
        || cfg.yolo.unwrap_or(false)
        || yolo::check_parent_cmdline_for_yolo();

    let s = Segments::new(data, t, home_path, full_yolo);

    let dot = &t.dot;

    let line1_wide = join_non_empty(&[&s.yolo_seg, &s.state_seg, &s.cycle_seg, &s.m_wide, &s.ctx_bar_wide, &s.dir_wide, &s.v_wide, &s.conv_wide, &s.tok_details_wide], dot);
    let line2_wide = join_non_empty(&[&s.art_wide, &s.sub_wide, &s.bg_wide, &s.sb_wide, &s.quota_wide], dot);

    let cols = data.terminal_width.unwrap_or(80);
    let target_cols = cols.saturating_sub(2);
    let margin = 8;

    let len1_wide = visible_len(&line1_wide);
    let len2_wide = visible_len(&line2_wide);

    let mut output_lines = Vec::new();

    if cols >= 135 && target_cols >= (len1_wide + len2_wide + margin) {
        output_lines.push(print_right_aligned(&line1_wide, &line2_wide, target_cols));
    }

    if output_lines.is_empty() {
        let r_candidate = |idx: usize| -> ((String, String), (String, String)) {
            let (p1, r1_right, ctx_seg) = match idx {
                0 => (
                    join_non_empty(&[&s.yolo_seg, &s.state_seg, &s.cycle_seg, &s.m_wide], dot),
                    join_non_empty(&[&s.art_wide, &s.sub_wide, &s.bg_wide, &s.sb_wide], dot),
                    s.ctx_bar_wide.as_str(),
                ),
                1 => (
                    join_non_empty(&[&s.yolo_seg, &s.state_seg, &s.cycle_seg, &s.m_wide], dot),
                    join_non_empty(&[&s.art_wide, &s.sub_wide, &s.bg_wide, &s.sb_wide], dot),
                    s.ctx_bar_med.as_str(),
                ),
                2 => (
                    join_non_empty(&[&s.yolo_seg, &s.state_seg, &s.cycle_seg, &s.m_med], dot),
                    join_non_empty(&[&s.art_wide, &s.sub_wide, &s.bg_wide, &s.sb_med], dot),
                    s.ctx_bar_med.as_str(),
                ),
                3 => (
                    join_non_empty(&[&s.yolo_seg, &s.state_seg, &s.cycle_seg, &s.m_med], dot),
                    join_non_empty(&[&s.art_narrow, &s.sub_narrow, &s.bg_narrow, &s.sb_narrow], "  "),
                    s.ctx_bar_narrow.as_str(),
                ),
                4 => (
                    join_non_empty(&[&s.yolo_seg, &s.state_seg, &s.cycle_seg, &s.m_narrow], dot),
                    join_non_empty(&[&s.art_narrow, &s.sub_narrow, &s.bg_narrow], "  "),
                    s.ctx_bar_narrow.as_str(),
                ),
                5 => (
                    join_non_empty(&[&s.yolo_seg, &s.state_seg, &s.cycle_seg, &s.m_narrow], dot),
                    String::new(),
                    "",
                ),
                _ => (
                    join_non_empty(&[&s.yolo_seg, &s.state_seg, &s.cycle_seg], dot),
                    String::new(),
                    "",
                ),
            };

            let (p2, r2_right, tok_seg) = match idx {
                0 => (
                    join_non_empty(&[&s.dir_wide, &s.v_wide, &s.conv_wide], dot),
                    s.quota_wide.as_str(),
                    s.tok_details_wide.as_str(),
                ),
                1 => (
                    join_non_empty(&[&s.dir_wide, &s.v_wide, &s.conv_wide], dot),
                    s.quota_wide.as_str(),
                    s.tok_details_med.as_str(),
                ),
                2 => (
                    join_non_empty(&[&s.dir_wide, &s.v_wide, &s.conv_wide], dot),
                    s.quota_med.as_str(),
                    s.tok_details_med.as_str(),
                ),
                3 => (
                    join_non_empty(&[&s.dir_med, &s.v_med, &s.conv_med], dot),
                    s.quota_med.as_str(),
                    s.tok_details_med.as_str(),
                ),
                4 => (
                    join_non_empty(&[&s.dir_med, &s.v_med, &s.conv_med], dot),
                    s.quota_med.as_str(),
                    "",
                ),
                5 => (
                    join_non_empty(&[&s.dir_narrow, &s.v_narrow], dot),
                    s.quota_med.as_str(),
                    "",
                ),
                6 => (
                    join_non_empty(&[&s.dir_narrow, &s.v_narrow], dot),
                    s.quota_narrow.as_str(),
                    "",
                ),
                7 => (
                    join_non_empty(&[&s.dir_narrow], dot),
                    s.quota_narrow.as_str(),
                    "",
                ),
                8 => (
                    join_non_empty(&[&s.dir_narrow], dot),
                    "",
                    "",
                ),
                _ => (
                    String::new(),
                    "",
                    "",
                ),
            };

            let p1_len = visible_len(&p1);
            let p2_len = visible_len(&p2);

            let (r1_left, r2_left) = if !ctx_seg.is_empty() && !tok_seg.is_empty() && p1_len > 0 && p2_len > 0 {
                let max_p = p1_len.max(p2_len);
                let p1_pad = pad_to_visible_len(&p1, max_p);
                let p2_pad = pad_to_visible_len(&p2, max_p);
                (
                    format!("{}{}{}", p1_pad, dot, ctx_seg),
                    format!("{}{}{}", p2_pad, dot, tok_seg),
                )
            } else {
                (
                    if ctx_seg.is_empty() { p1 } else if p1.is_empty() { ctx_seg.to_string() } else { format!("{}{}{}", p1, dot, ctx_seg) },
                    if tok_seg.is_empty() { p2 } else if p2.is_empty() { tok_seg.to_string() } else { format!("{}{}{}", p2, dot, tok_seg) },
                )
            };

            ((r1_left, r1_right), (r2_left, r2_right.to_string()))
        };

        // Candidate 9 is the last resort (state only, nothing right-aligned); anything
        // still too wide after that is handled by the truncate below.
        let ((r1_left, r1_right), (r2_left, r2_right)) = (0..9)
            .map(r_candidate)
            .find(|((l1, r1), (l2, r2))| {
                let l1_vis = visible_len(l1);
                let r1_vis = visible_len(r1);
                let req1 = if r1_vis == 0 { l1_vis } else { l1_vis + 1 + r1_vis };

                let l2_vis = visible_len(l2);
                let r2_vis = visible_len(r2);
                let req2 = if r2_vis == 0 { l2_vis } else { l2_vis + 1 + r2_vis };

                req1 <= target_cols && req2 <= target_cols
            })
            .unwrap_or_else(|| r_candidate(9));

        let mut line1_str = print_right_aligned(&r1_left, &r1_right, target_cols);
        if visible_len(&line1_str) > target_cols {
            line1_str = truncate_to_visible_width(&line1_str, target_cols);
        }

        let mut line2_str = print_right_aligned(&r2_left, &r2_right, target_cols);
        if visible_len(&line2_str) > target_cols {
            line2_str = truncate_to_visible_width(&line2_str, target_cols);
        }

        output_lines.push(line1_str);
        output_lines.push(line2_str);
    }

    output_lines
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ansi::visible_len;
    use crate::yolo::detect_yolo_in_json;

    #[test]
    fn test_dynamic_line_width_truncation_across_widths() {
        let widths = [80, 100, 115, 135, 160];
        let cfg = Config::default();

        // Derived rather than spelled out: this test is about how many columns the rendered line
        // occupies, so the only thing the path has to be is a real absolute one for the platform.
        let home = std::env::temp_dir().join("agy-home");
        let cwd = home.join("code").join("project");

        let raw_val = serde_json::json!({
            "agent_state": "working",
            "cwd": cwd.to_string_lossy(),
            "conversation_id": "1234567890abcdef",
            "context_window": {
                "used_percentage": 45.5,
                "total_input_tokens": 45000,
                "total_output_tokens": 10000,
                "context_window_size": 1000000
            },
            "artifact_count": 3,
            "task_count": 1,
            "subagents": 2,
            "model": {
                "id": "gemini-1.5-pro",
                "effort": "high"
            }
        });

        let yolo_from_json = detect_yolo_in_json(&raw_val);

        for &cols in &widths {
            let mut data: InputData = serde_json::from_value(raw_val.clone()).unwrap();
            data.terminal_width = Some(cols);

            let t = Theme::new(&data, &cfg);
            let lines = render_statusline(&data, &cfg, &t, yolo_from_json, &home.to_string_lossy());
            assert!(!lines.is_empty());
            assert!(lines.len() <= 2);

            let target_cols = cols.saturating_sub(2);
            for line in &lines {
                let vis = visible_len(line);
                assert!(
                    vis <= target_cols,
                    "Line visible width {} exceeds target_cols {} at total width {}",
                    vis,
                    target_cols,
                    cols
                );
            }
        }
    }
}
