use async_openai::types::responses as openai;
use serde::{Deserialize, Serialize};
use structural_convert::StructuralConvert;
use strum::Display;

// ============================================================
// Tool Definition Supporting Types
// ============================================================

#[derive(Debug, Clone, PartialEq, Eq, StructuralConvert, Serialize, Deserialize)]
#[convert(from(openai::CodeInterpreterContainerAuto))]
pub struct CodeInterpreterContainerAuto {
    pub file_ids: Option<Vec<String>>,
    pub memory_limit: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, StructuralConvert, Serialize, Deserialize)]
#[convert(from(openai::CodeInterpreterToolContainer))]
pub enum CodeInterpreterToolContainer {
    Auto(CodeInterpreterContainerAuto),
    ContainerID(String),
}

// ============================================================
// Tool Definition
// ============================================================

#[derive(Debug, Clone, PartialEq, Eq, StructuralConvert, Serialize, Deserialize)]
#[convert(from(openai::CodeInterpreterTool))]
pub struct CodeInterpreterTool {
    pub container: CodeInterpreterToolContainer,
}

// ============================================================
// Output / Resource Supporting Types
// ============================================================

#[derive(Debug, Clone, PartialEq, Eq, StructuralConvert, Serialize, Deserialize)]
#[convert(from(openai::CodeInterpreterOutputLogs))]
pub struct CodeInterpreterOutputLogs {
    pub logs: String,
}

#[derive(Debug, Clone, PartialEq, Eq, StructuralConvert, Serialize, Deserialize)]
#[convert(from(openai::CodeInterpreterOutputImage))]
pub struct CodeInterpreterOutputImage {
    pub url: String,
}

// The upstream SDK also exposes `CodeInterpreterFile`, but the current
// `CodeInterpreterToolCallOutput` response shape here only exposes `Logs` and
// `Image` variants, so we do not model a separate local file output type yet.
#[derive(Debug, Clone, PartialEq, Eq, StructuralConvert, Serialize, Deserialize)]
#[convert(from(openai::CodeInterpreterToolCallOutput))]
pub enum CodeInterpreterToolCallOutput {
    Logs(CodeInterpreterOutputLogs),
    Image(CodeInterpreterOutputImage),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, StructuralConvert, Display, Serialize, Deserialize)]
#[convert(from(openai::CodeInterpreterToolCallStatus))]
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

#[derive(Debug, Clone, PartialEq, Eq, StructuralConvert, Serialize, Deserialize)]
#[convert(from(openai::CodeInterpreterToolCall))]
pub struct CodeInterpreterToolCall {
    pub code: Option<String>,
    pub container_id: String,
    pub id: String,
    pub outputs: Option<Vec<CodeInterpreterToolCallOutput>>,
    pub status: CodeInterpreterToolCallStatus,
}
