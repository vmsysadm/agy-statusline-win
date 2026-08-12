use unicode_width::UnicodeWidthStr;

/// Convert hex color codes (#RRGGBB or #RGB) in a string to ANSI escape sequences.
pub(crate) fn convert_color_value(val: &str) -> String {
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

/// Remove ANSI escape sequences from a string.
pub(crate) fn strip_ansi(s: &str) -> String {
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

/// Get the visible column width of a string (ANSI codes excluded).
pub(crate) fn visible_len(s: &str) -> usize {
    let clean = strip_ansi(s);
    UnicodeWidthStr::width(clean.as_str())
}

/// Truncate a string to a maximum visible width. Keep ANSI codes intact.
pub(crate) fn truncate_to_visible_width(s: &str, max_width: usize) -> String {
    let mut out = String::with_capacity(s.len());
    let mut cur_width = 0;
    let mut in_esc = false;
    let mut esc_buf = String::new();

    for c in s.chars() {
        if c == '\x1b' {
            in_esc = true;
            esc_buf.push(c);
            continue;
        }
        if in_esc {
            esc_buf.push(c);
            if c == 'm' {
                in_esc = false;
                out.push_str(&esc_buf);
                esc_buf.clear();
            }
            continue;
        }

        let w = unicode_width::UnicodeWidthChar::width(c).unwrap_or(0);
        if cur_width + w > max_width {
            break;
        }
        out.push(c);
        cur_width += w;
    }

    out.push_str("\x1b[0m");
    out
}

/// Format a large number into a human-readable string (K/M suffixes).
pub(crate) fn format_human(num: u64) -> String {
    if num >= 1_000_000 {
        format!("{:.1}M", num as f64 / 1_000_000.0)
    } else if num >= 1_000 {
        format!("{:.1}K", num as f64 / 1_000.0)
    } else {
        num.to_string()
    }
}

/// Format seconds into a compact duration string (e.g. "1h5m", "3m", "45s").
pub(crate) fn format_seconds(s: u64) -> String {
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

/// Split a string into lowercase alphanumeric tokens.
pub(crate) fn get_tokens(s: &str) -> Vec<String> {
    s.to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .filter(|t| !t.is_empty())
        .map(|t| t.to_string())
        .collect()
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
    fn test_unicode_width_visible_len() {
        let emoji_str = "🟢 Ready";
        // 🟢 has column width 2, " Ready" has width 6 => total width 8
        assert_eq!(visible_len(emoji_str), 8);
    }

    #[test]
    fn test_truncate_to_visible_width() {
        let text = "\x1b[31mHello World\x1b[0m";
        let truncated = truncate_to_visible_width(text, 5);
        assert_eq!(visible_len(&truncated), 5);

        let ascii_text = "Standard Text";
        let truncated_ascii = truncate_to_visible_width(ascii_text, 8);
        assert_eq!(visible_len(&truncated_ascii), 8);
    }
}
