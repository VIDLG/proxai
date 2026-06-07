use serde::{Deserialize, Serialize};

use std::collections::HashMap;

use super::super::OutputStatus;

// ============================================================
// Local Shell Supporting Types
// ============================================================

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LocalShellExecAction {
    pub command: Vec<String>,
    pub env: HashMap<String, String>,
    pub timeout_ms: Option<u64>,
    pub user: Option<String>,
    pub working_directory: Option<String>,
}

// ============================================================
// Input / Context Item Shapes
// ============================================================

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LocalShellToolCallOutput {
    pub id: String,
    pub output: String,
    pub status: Option<OutputStatus>,
}

// ============================================================
// Local Shell Output Shapes
// ============================================================

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LocalShellToolCall {
    pub action: LocalShellExecAction,
    pub call_id: String,
    pub id: String,
    pub status: OutputStatus,
}
