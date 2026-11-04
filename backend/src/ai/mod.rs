//! AI Orchestrator module.
//! Integrates Antigravity CLI as a cloud AI engine using a long-lived process via stdin/stdout
//! and a strict Tool Registry for device operations.

pub mod agy_manager;
pub mod client;
pub mod models;
pub mod tools;

pub use agy_manager::{
    AgyLoginChallenge, AgyLoginResult, AgyManager, AgyPaths, AgyStatus, SubmitCodeRequest,
};
pub use client::{AiClient, AiOrchestrator, AiReadiness, CliAiClient, MockAiClient};
pub use models::{
    AiPromptRequest, AiResponse, AiSessionMetadata, SessionMessage, ToolCallRequest,
    ToolCallResult, ToolDeclaration,
};
pub use tools::{Tool, ToolRegistry};
