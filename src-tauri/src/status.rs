use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StatusField<T> { pub value: Option<T>, pub source: Option<String>, pub observed_at: Option<String>, pub confidence: Option<f32>, pub stale: bool }
impl<T> Default for StatusField<T> { fn default() -> Self { Self { value: None, source: None, observed_at: None, confidence: None, stale: false } } }

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CodexStatus {
    pub username: StatusField<String>, pub model: StatusField<String>, pub reasoning_effort: StatusField<String>,
    pub reasoning_speed: StatusField<f64>, pub speed_mode: StatusField<String>, pub subscription: StatusField<String>, pub remaining_percent: StatusField<f64>,
    pub reset_at: StatusField<String>, pub client_version: StatusField<String>, pub monthly_usage: StatusField<f64>,
    pub weekly_duration_seconds: StatusField<f64>, pub sync_state: String, pub message: Option<String>,
}
impl CodexStatus {
    pub fn unavailable(message: impl Into<String>) -> Self { Self { sync_state: "recon-required".into(), message: Some(message.into()), ..Self::default() } }
    pub fn unsupported(message: impl Into<String>) -> Self { Self { sync_state: "unsupported".into(), message: Some(message.into()), ..Self::default() } }
}
