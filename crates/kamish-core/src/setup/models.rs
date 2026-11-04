use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StepStatus {
    Pending,
    InProgress,
    Completed,
    Skipped,
    Degraded,
}

impl StepStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            StepStatus::Pending => "pending",
            StepStatus::InProgress => "in_progress",
            StepStatus::Completed => "completed",
            StepStatus::Skipped => "skipped",
            StepStatus::Degraded => "degraded",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepHealthInfo {
    pub healthy: bool,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetupStepInfo {
    pub id: String,
    pub title: String,
    pub status: StepStatus,
    pub is_mandatory: bool,
    pub health: StepHealthInfo,
    pub data: Option<Value>,
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DegradedComponentInfo {
    pub component: String,
    pub title: String,
    pub recovery_step: String,
    pub recovery_url: String,
    pub error_message: String,
    pub severity: String,
    pub can_fallback: bool,
    pub fallback_action_description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetupStatusResponse {
    pub is_completed: bool,
    pub current_step: String,
    pub completed_at: Option<String>,
    pub steps: Vec<SetupStepInfo>,
    pub degraded_components: Vec<DegradedComponentInfo>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SaveStepRequest {
    pub step: String,
    pub status: Option<StepStatus>,
    pub data: Option<Value>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SaveStepResponse {
    pub success: bool,
    pub step: String,
    pub status: StepStatus,
    pub next_step: String,
    pub is_completed: bool,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct CompleteSetupResponse {
    pub success: bool,
    pub is_completed: bool,
    pub completed_at: String,
    pub message: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SimulateDegradationRequest {
    pub component: String,
    pub degraded: bool,
    pub error_reason: Option<String>,
}
