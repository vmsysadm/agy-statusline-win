use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

#[derive(Deserialize, Debug, Default)]
pub(crate) struct ConfigColors {
    pub(crate) reset: Option<String>,
    pub(crate) bold: Option<String>,
    pub(crate) italic: Option<String>,
    pub(crate) foreground: Option<HashMap<String, String>>,
    pub(crate) ui: Option<HashMap<String, String>>,
}


fn default_true() -> bool {
    true
}

#[derive(Deserialize, Debug, Clone)]
pub(crate) struct TerminalTitleConfig {
    #[serde(default = "default_true")]
    pub(crate) enabled: bool,
    #[serde(default)]
    pub(crate) osc7_cwd: bool,
    /// Alternate the working icon between the high- and low-brightness symbols so the tab
    /// appears to glow whenever this agy session is running -- that is, whenever its own
    /// `agent_state` is working. It is not tied to subagents or background tasks, which
    /// report separately through their own indicators. Set false for a steady icon.
    #[serde(default = "default_true")]
    pub(crate) pulse: bool,
}

impl Default for TerminalTitleConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            osc7_cwd: false,
            pulse: true,
        }
    }
}

#[derive(Deserialize, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Config {
    pub(crate) colors: Option<ConfigColors>,
    pub(crate) icons: Option<serde_json::Value>,
    #[serde(default)]
    pub(crate) yolo: Option<bool>,
    #[serde(default, alias = "use_ascii")]
    pub(crate) use_ascii: Option<bool>,
    #[serde(default, alias = "use_nerd_fonts", alias = "nerd_fonts")]
    pub(crate) use_nerd_fonts: Option<bool>,
    #[serde(default, alias = "terminal_title")]
    pub(crate) terminal_title: Option<TerminalTitleConfig>,
}

/// Load the statusline config from the user's home directory.
pub(crate) fn load_config(home_path: &str) -> Config {
    let config_path = PathBuf::from(home_path)
        .join(".gemini")
        .join("antigravity-cli")
        .join("statusline_config.json");

    if config_path.exists() {
        if let Ok(cfg_content) = fs::read_to_string(&config_path) {
            return serde_json::from_str(&cfg_content).unwrap_or_default();
        }
    }
    Config::default()
}
