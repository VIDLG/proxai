use serde::{Deserialize, Serialize};
use strum::Display;

// ============================================================
// Tool Definition Supporting Types
// ============================================================

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CodeInterpreterContainerAuto {
    pub file_ids: Option<Vec<String>>,
    pub memory_limit: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CodeInterpreterToolContainer {
    Auto(CodeInterpreterContainerAuto),
    ContainerID(String),
}

// ============================================================
// Tool Definition
// ============================================================

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CodeInterpreterTool {
    pub container: CodeInterpreterToolContainer,
}

// ============================================================
// Output / Resource Supporting Types
// ============================================================

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CodeInterpreterOutputLogs {
    pub logs: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CodeInterpreterOutputImage {
    pub url: String,
}

// The upstream SDK also exposes `CodeInterpreterFile`, but the current
// `CodeInterpreterToolCallOutput` response shape here only exposes `Logs` and
// `Image` variants, so we do not model a separate local file output type yet.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CodeInterpreterToolCallOutput {
    Logs(CodeInterpreterOutputLogs),
    Image(CodeInterpreterOutputImage),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Display, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum CodeInterpreterToolCallStatus {
    InProgress,
    Completed,
    Incomplete,
    Interpreting,
    Failed,
}

// ============================================================
// Output / Resource Shapes
// ============================================================

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CodeInterpreterToolCall {
    pub code: Option<String>,
    pub container_id: String,
    pub id: String,
    pub outputs: Option<Vec<CodeInterpreterToolCallOutput>>,
    pub status: CodeInterpreterToolCallStatus,
}
