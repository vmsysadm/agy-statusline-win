use crate::ansi::{truncate_to_visible_width, visible_len};
use crate::config::Config;
use crate::data::InputData;
use crate::segments::Segments;
use crate::theme::Theme;
use crate::yolo;

/// Join non-empty items with a separator.
fn join_with_dot(items: &[&str], dot: &str) -> String {
    let mut result = String::new();
    let mut first = true;
    for &item in items {
        if !item.is_empty() {
            if !first {
                result.push_str(dot);
            }
            result.push_str(item);
            first = false;
        }
    }
    result
}

/// Join non-empty items with double spaces.
fn join_with_space(items: &[&str]) -> String {
    let mut result = String::new();
    let mut first = true;
    for &item in items {
        if !item.is_empty() {
            if !first {
                result.push_str("  ");
            }
            result.push_str(item);
            first = false;
        }
    }
    result
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

    let mut out = String::with_capacity(left.len() + pad + right.len());
    out.push_str(left);
    for _ in 0..pad {
        out.push(' ');
    }
    out.push_str(right);
    out
}

/// Build the final statusline output (1 or 2 lines).
pub(crate) fn render_statusline(
    data: &InputData,
    cfg: &Config,
    yolo_from_json: bool,
    raw_use_ascii: bool,
    home_path: &str,
) -> Vec<String> {
    let t = Theme::new(data, cfg, raw_use_ascii);

    // Combine config-level and data-level YOLO flags
    let cfg_yolo = cfg.show_yolo.unwrap_or(false)
        || cfg.always_show_yolo.unwrap_or(false)
        || cfg.yolo.unwrap_or(false);

    let full_yolo = yolo_from_json
        || yolo::check_parent_cmdline_for_yolo()
        || cfg_yolo;

    let s = Segments::new(data, &t, home_path, full_yolo);

    let dot = &t.dot;

    let line1_wide = join_with_dot(&[&s.yolo_seg, &s.state_seg, &s.cycle_seg, &s.m_wide, &s.dir_wide, &s.v_wide, &s.conv_wide], dot);
    let ctx_tok_wide = format!("{}{}", s.ctx_bar_wide, s.tok_details_wide);
    let line2_wide = join_with_dot(&[&s.art_wide, &s.sub_wide, &s.bg_wide, &s.sb_wide, &ctx_tok_wide, &s.quota_wide], dot);

    let cols = data.terminal_width.unwrap_or(80);
    let target_cols = cols.saturating_sub(2);
    let margin = 8;

    let len1_wide = visible_len(&line1_wide);
    let len2_wide = visible_len(&line2_wide);

    let mut output_lines = Vec::new();

    if cols >= 135 && target_cols >= (len1_wide + len2_wide + margin) {
        let single_line = print_right_aligned(&line1_wide, &line2_wide, target_cols);
        if visible_len(&single_line) <= target_cols {
            output_lines.push(single_line);
        }
    }

    if output_lines.is_empty() {
        let r1_candidate = |idx: usize| -> (String, String) {
            match idx {
                0 => (
                    join_with_dot(&[&s.yolo_seg, &s.state_seg, &s.cycle_seg, &s.m_wide], dot),
                    join_with_dot(&[&s.art_wide, &s.sub_wide, &s.bg_wide, &s.sb_wide], dot),
                ),
                1 => (
                    join_with_dot(&[&s.yolo_seg, &s.state_seg, &s.cycle_seg, &s.m_wide], dot),
                    join_with_dot(&[&s.art_med, &s.sub_med, &s.bg_med, &s.sb_med], dot),
                ),
                2 => (
                    join_with_dot(&[&s.yolo_seg, &s.state_seg, &s.cycle_seg, &s.m_med], dot),
                    join_with_dot(&[&s.art_med, &s.sub_med, &s.bg_med, &s.sb_med], dot),
                ),
                3 => (
                    join_with_dot(&[&s.yolo_seg, &s.state_seg, &s.cycle_seg, &s.m_med], dot),
                    join_with_space(&[&s.art_narrow, &s.sub_narrow, &s.bg_narrow, &s.sb_narrow]),
                ),
                4 => (
                    join_with_dot(&[&s.yolo_seg, &s.state_seg, &s.cycle_seg, &s.m_narrow], dot),
                    join_with_space(&[&s.art_narrow, &s.sub_narrow, &s.bg_narrow]),
                ),
                5 => (
                    join_with_dot(&[&s.yolo_seg, &s.state_seg, &s.cycle_seg, &s.m_narrow], dot),
                    String::new(),
                ),
                _ => (
                    join_with_dot(&[&s.yolo_seg, &s.state_seg, &s.cycle_seg], dot),
                    String::new(),
                ),
            }
        };

        let (r1_left, r1_right) = (0..7)
            .map(r1_candidate)
            .find(|(l, r)| {
                let l_vis = visible_len(l);
                let r_vis = visible_len(r);
                let req_len = if r_vis == 0 { l_vis } else { l_vis + 1 + r_vis };
                req_len <= target_cols
            })
            .unwrap_or_else(|| r1_candidate(6));

        let mut line1_str = print_right_aligned(&r1_left, &r1_right, target_cols);
        if visible_len(&line1_str) > target_cols {
            line1_str = truncate_to_visible_width(&line1_str, target_cols);
        }

        let ctx_tok_w_m = format!("{}{}", s.ctx_bar_wide, s.tok_details_med);
        let ctx_tok_m_m = format!("{}{}", s.ctx_bar_med, s.tok_details_med);

        let r2_candidate = |idx: usize| -> (String, String) {
            match idx {
                0 => (
                    join_with_dot(&[&s.dir_wide, &s.v_wide, &s.conv_wide], dot),
                    join_with_dot(&[&ctx_tok_wide, &s.quota_wide], dot),
                ),
                1 => (
                    join_with_dot(&[&s.dir_wide, &s.v_wide, &s.conv_wide], dot),
                    join_with_dot(&[&ctx_tok_w_m, &s.quota_wide], dot),
                ),
                2 => (
                    join_with_dot(&[&s.dir_wide, &s.v_wide, &s.conv_wide], dot),
                    join_with_dot(&[&ctx_tok_m_m, &s.quota_med], dot),
                ),
                3 => (
                    join_with_dot(&[&s.dir_med, &s.v_med, &s.conv_med], dot),
                    join_with_dot(&[&ctx_tok_m_m, &s.quota_med], dot),
                ),
                4 => (
                    join_with_dot(&[&s.dir_med, &s.v_med, &s.conv_med], dot),
                    join_with_dot(&[&s.ctx_bar_med, &s.quota_med], dot),
                ),
                5 => (
                    join_with_dot(&[&s.dir_narrow, &s.v_narrow], dot),
                    join_with_dot(&[&s.ctx_bar_med, &s.quota_med], dot),
                ),
                6 => (
                    join_with_dot(&[&s.dir_narrow, &s.v_narrow], dot),
                    join_with_dot(&[&s.ctx_bar_narrow, &s.quota_narrow], dot),
                ),
                7 => (
                    join_with_dot(&[&s.dir_narrow], dot),
                    join_with_dot(&[&s.ctx_bar_narrow, &s.quota_narrow], dot),
                ),
                8 => (
                    join_with_dot(&[&s.dir_narrow], dot),
                    s.ctx_bar_narrow.clone(),
                ),
                _ => (
                    String::new(),
                    s.ctx_bar_narrow.clone(),
                ),
            }
        };

        let (r2_left, r2_right) = (0..10)
            .map(r2_candidate)
            .find(|(l, r)| {
                let l_vis = visible_len(l);
                let r_vis = visible_len(r);
                let req_len = if r_vis == 0 { l_vis } else { l_vis + 1 + r_vis };
                req_len <= target_cols
            })
            .unwrap_or_else(|| r2_candidate(9));

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
        let raw_val = serde_json::json!({
            "agent_state": "working",
            "cwd": "C:\\Users\\billy\\code\\project",
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
        let raw_use_ascii = raw_val.get("mode").and_then(|v| v.as_str()).map(|m| m.eq_ignore_ascii_case("ascii")).unwrap_or(false)
            || raw_val.get("use_ascii").and_then(|v| v.as_bool()).unwrap_or(false)
            || raw_val.get("useAscii").and_then(|v| v.as_bool()).unwrap_or(false);

        for &cols in &widths {
            let mut data: InputData = serde_json::from_value(raw_val.clone()).unwrap();
            data.terminal_width = Some(cols);

            let lines = render_statusline(&data, &cfg, yolo_from_json, raw_use_ascii, "C:\\Users\\billy");
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
