use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

#[derive(Deserialize, Debug, Default)]
pub(crate) struct ConfigColors {
    pub(crate) reset: Option<String>,
    pub(crate) bold: Option<String>,
    pub(crate) dim: Option<String>,
    pub(crate) italic: Option<String>,
    pub(crate) foreground: Option<HashMap<String, String>>,
    pub(crate) ui: Option<HashMap<String, String>>,
}


#[derive(Deserialize, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Config {
    pub(crate) colors: Option<ConfigColors>,
    pub(crate) icons: Option<serde_json::Value>,
    #[serde(default, alias = "show_yolo")]
    pub(crate) show_yolo: Option<bool>,
    #[serde(default, alias = "always_show_yolo")]
    pub(crate) always_show_yolo: Option<bool>,
    #[serde(default)]
    pub(crate) yolo: Option<bool>,
    #[serde(default, alias = "use_ascii")]
    pub(crate) use_ascii: Option<bool>,
    #[serde(default, alias = "use_nerd_fonts", alias = "useNerdFonts", alias = "nerd_fonts")]
    pub(crate) use_nerd_fonts: Option<bool>,
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
