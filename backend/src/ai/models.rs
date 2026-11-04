//! AI data models and schemas.

use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Request payload for POST /api/v1/ai/prompt
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiPromptRequest {
    pub prompt: String,
    pub session_id: Option<String>,
}

/// A tool call invocation requested by the AI model or parser
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ToolCallRequest {
    pub name: String,
    pub arguments: Value,
}

/// Result of executing a registered tool
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ToolCallResult {
    pub tool: String,
    pub arguments: Value,
    pub result: Value,
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Complete response from AI endpoint
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AiResponse {
    pub response: String,
    pub tool_calls: Vec<ToolCallResult>,
    pub session_id: String,
}

/// JSON Schema declaration for an individual tool
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ToolDeclaration {
    pub name: String,
    pub description: String,
    pub parameters: Value,
}

/// Single message in AI session history
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionMessage {
    pub role: String, // "user" or "assistant"
    pub content: String,
    pub timestamp: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCallResult>>,
}

/// Session metadata stored in ai_sessions.metadata_json
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AiSessionMetadata {
    pub messages: Vec<SessionMessage>,
    pub last_active_at: String,
}

impl AiSessionMetadata {
    pub fn add_user_message(&mut self, text: &str) {
        self.messages.push(SessionMessage {
            role: "user".to_string(),
            content: text.to_string(),
            timestamp: Utc::now().to_rfc3339(),
            tool_calls: None,
        });
        self.last_active_at = Utc::now().to_rfc3339();
    }

    pub fn add_assistant_message(&mut self, text: &str, tool_calls: Vec<ToolCallResult>) {
        let tc = if tool_calls.is_empty() {
            None
        } else {
            Some(tool_calls)
        };
        self.messages.push(SessionMessage {
            role: "assistant".to_string(),
            content: text.to_string(),
            timestamp: Utc::now().to_rfc3339(),
            tool_calls: tc,
        });
        self.last_active_at = Utc::now().to_rfc3339();
    }
}
