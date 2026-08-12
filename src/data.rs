use serde::Deserialize;
use std::collections::HashMap;

#[derive(Deserialize, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ContextWindow {
    #[serde(default, alias = "used_percentage")]
    pub(crate) used_percentage: Option<f64>,
    #[serde(default, alias = "total_input_tokens")]
    pub(crate) total_input_tokens: Option<u64>,
    #[serde(default, alias = "total_output_tokens")]
    pub(crate) total_output_tokens: Option<u64>,
    #[serde(default, alias = "context_window_size")]
    pub(crate) context_window_size: Option<u64>,
}

#[derive(Deserialize, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Sandbox {
    #[serde(default)]
    pub(crate) enabled: Option<bool>,
    #[serde(default, alias = "allow_network")]
    pub(crate) allow_network: Option<bool>,
}

#[derive(Deserialize, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Model {
    #[serde(default)]
    pub(crate) id: Option<String>,
    #[serde(default, alias = "display_name")]
    pub(crate) display_name: Option<String>,
    #[serde(default)]
    pub(crate) effort: Option<String>,
    #[serde(default, alias = "effort_level")]
    pub(crate) effort_level: Option<String>,
}

#[derive(Deserialize, Debug, Default, Clone)]
#[serde(rename_all = "camelCase")]
pub(crate) struct QuotaEntry {
    #[serde(default, alias = "remaining_fraction")]
    pub(crate) remaining_fraction: Option<f64>,
    #[serde(default, alias = "reset_in_seconds")]
    pub(crate) reset_in_seconds: Option<u64>,
}

#[derive(Deserialize, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub(crate) struct InputData {
    #[serde(default, alias = "agent_state")]
    pub(crate) agent_state: Option<String>,
    #[serde(default, alias = "context_window")]
    pub(crate) context_window: Option<ContextWindow>,
    #[serde(default)]
    pub(crate) sandbox: Option<Sandbox>,
    #[serde(default, alias = "artifact_count")]
    pub(crate) artifact_count: Option<u64>,
    #[serde(default, alias = "task_count")]
    pub(crate) task_count: Option<u64>,
    #[serde(default)]
    pub(crate) subagents: Option<serde_json::Value>,
    #[serde(default)]
    pub(crate) model: Option<Model>,
    #[serde(default, alias = "terminal_width")]
    pub(crate) terminal_width: Option<usize>,
    #[serde(default)]
    pub(crate) cwd: Option<String>,
    #[serde(default, alias = "conversation_id")]
    pub(crate) conversation_id: Option<String>,
    #[serde(default, alias = "cycle_mode")]
    pub(crate) cycle_mode: Option<String>,
    #[serde(default, alias = "nerd_fonts_supported")]
    pub(crate) nerd_fonts_supported: Option<bool>,
    #[serde(default)]
    pub(crate) quota: Option<HashMap<String, QuotaEntry>>,
    #[serde(default, alias = "auto_approve")]
    pub(crate) auto_approve: Option<bool>,
    #[serde(default, alias = "auto_approve_enabled")]
    pub(crate) auto_approve_enabled: Option<bool>,
    #[serde(default, alias = "dangerously_skip_permissions")]
    pub(crate) dangerously_skip_permissions: Option<bool>,
    #[serde(default, alias = "skip_permissions")]
    pub(crate) skip_permissions: Option<bool>,
    #[serde(default, alias = "approval_mode")]
    pub(crate) approval_mode: Option<String>,
    #[serde(default)]
    pub(crate) mode: Option<String>,
    #[serde(default, alias = "use_ascii")]
    pub(crate) use_ascii: Option<bool>,
    #[serde(default)]
    pub(crate) yolo: Option<bool>,
    #[serde(default, alias = "is_yolo")]
    pub(crate) is_yolo: Option<bool>,
}

#[derive(Clone, Debug)]
pub(crate) struct MatchedQuota {
    pub(crate) key: String,
    pub(crate) remaining_fraction: f64,
    pub(crate) reset_in_seconds: u64,
    pub(crate) score: f64,
}
