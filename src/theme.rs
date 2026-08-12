use std::env;

use crate::ansi::convert_color_value;
use crate::config::Config;
use crate::data::InputData;

/// Resolved theme holding all ANSI color strings and icon strings.
#[allow(dead_code)]
pub(crate) struct Theme {
    pub(crate) reset: String,
    pub(crate) bold: String,
    pub(crate) _dim: String,
    pub(crate) italic: String,

    pub(crate) fg_red: String,
    pub(crate) fg_green: String,
    pub(crate) fg_yellow: String,
    pub(crate) fg_blue: String,
    pub(crate) fg_magenta: String,
    pub(crate) fg_cyan: String,
    pub(crate) fg_white: String,
    pub(crate) fg_gray: String,

    pub(crate) fg_bright_red: String,
    pub(crate) fg_bright_green: String,
    pub(crate) fg_bright_yellow: String,
    pub(crate) fg_bright_blue: String,
    pub(crate) fg_bright_cyan: String,
    pub(crate) fg_bright_magenta: String,
    pub(crate) fg_bright_white: String,

    pub(crate) num_color: String,
    pub(crate) dot: String,

    pub(crate) use_ascii: bool,
    pub(crate) use_nerd_fonts: bool,

    // Icons — state
    pub(crate) icon_ready: String,
    pub(crate) icon_thinking: String,
    pub(crate) icon_working: String,
    pub(crate) icon_tool: String,
    pub(crate) icon_unknown: String,

    // Icons — components
    pub(crate) icon_folder: String,
    pub(crate) icon_model: String,
    pub(crate) icon_branch: String,
    pub(crate) icon_conv: String,
    pub(crate) icon_ctx: String,
    pub(crate) icon_tok: String,
    pub(crate) icon_art: String,
    pub(crate) icon_sub: String,
    pub(crate) icon_bg: String,

    // Icons — sandbox
    pub(crate) icon_sb_net: String,
    pub(crate) icon_sb_nonet: String,
    pub(crate) icon_sb_off: String,

    // Icons — cycle
    pub(crate) icon_cycle_accept: String,
    pub(crate) icon_cycle_plan: String,

    // Icons — other
    pub(crate) icon_yolo: String,
}

impl Theme {
    /// Build a fully resolved theme from input data and config.
    pub(crate) fn new(data: &InputData, cfg: &Config, raw_use_ascii: bool) -> Self {
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
                if let Ok(v) = env::var("USE_ASCII") {
                    v == "true" || v == "1"
                } else if let Ok(v) = env::var("USE_NERD_FONTS") {
                    v == "false" || v == "0"
                } else if cfg.use_nerd_fonts.unwrap_or(false) {
                    false
                } else {
                    let mode_ascii = data.mode.as_deref().map(|m| m.eq_ignore_ascii_case("ascii")).unwrap_or(false);
                    mode_ascii || raw_use_ascii || true
                }
            })
        });

        let use_nerd_fonts = if use_ascii {
            false
        } else {
            data.nerd_fonts_supported.unwrap_or_else(|| {
                cfg.use_nerd_fonts.unwrap_or_else(|| {
                    env::var("USE_NERD_FONTS").map(|v| v != "false" && v != "0").unwrap_or(true)
                })
            })
        };

        let cfg_icons = cfg.icons.as_ref().and_then(|i| {
            if use_ascii {
                i.get("ascii").cloned()
            } else if use_nerd_fonts {
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
        let icon_model = get_icon_str(&cfg_icons, &cfg.icons, cat_comp, "model", if use_ascii { "" } else if use_nerd_fonts { "\u{f400}" } else { "💡" });
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

        Theme {
            reset, bold, _dim, italic,
            fg_red, fg_green, fg_yellow, fg_blue, fg_magenta, fg_cyan, fg_white, fg_gray,
            fg_bright_red, fg_bright_green, fg_bright_yellow, fg_bright_blue,
            fg_bright_cyan, fg_bright_magenta, fg_bright_white,
            num_color, dot,
            use_ascii, use_nerd_fonts,
            icon_ready, icon_thinking, icon_working, icon_tool, icon_unknown,
            icon_folder, icon_model, icon_branch, icon_conv, icon_ctx, icon_tok,
            icon_art, icon_sub, icon_bg,
            icon_sb_net, icon_sb_nonet, icon_sb_off,
            icon_cycle_accept, icon_cycle_plan,
            icon_yolo,
        }
    }
}

/// Look up an icon string from the icon config, with fallback chains.
pub(crate) fn get_icon_str(
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
            if category != "nerd_fonts" && category != "emoji_fallback" && category != "ascii" {
                if let Some(cat) = val.get(category) {
                    if let Some(s) = cat.get(k).and_then(|v| v.as_str()) {
                        return s.to_string();
                    }
                }
            }
            if k != "nerd_fonts" && k != "emoji_fallback" && k != "ascii" {
                if let Some(s) = val.get(k).and_then(|v| v.as_str()) {
                    return s.to_string();
                }
            }
        }
    }
    default_str.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_icon_str_flat_lookup() {
        let active_icons: Option<serde_json::Value> = None;
        let flat_icons: Option<serde_json::Value> = serde_json::from_str(r#"{"ready": "[READY]"}"#).ok();
        let icon = get_icon_str(&active_icons, &flat_icons, "state", "ready", "*");
        assert_eq!(icon, "[READY]");
    }
}
