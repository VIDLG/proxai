use async_openai::types::responses as openai;
use serde_json::Value;
use structural_convert::StructuralConvert;
use strum::Display;

use std::collections::HashMap;

use super::super::Filter;

// ============================================================
// Tool Definition Supporting Types
// ============================================================

#[derive(Debug, Clone, PartialEq, StructuralConvert)]
#[convert(from(openai::HybridSearch))]
pub struct HybridSearch {
    pub embedding_weight: f32,
    pub text_weight: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, StructuralConvert, Display)]
#[convert(from(openai::RankVersionType))]
pub enum RankVersionType {
    Auto,
    #[strum(to_string = "default-2024-11-15")]
    Default20241115,
}

#[derive(Debug, Clone, PartialEq, StructuralConvert)]
#[convert(from(openai::RankingOptions))]
pub struct RankingOptions {
    pub hybrid_search: Option<HybridSearch>,
    pub ranker: RankVersionType,
    pub score_threshold: Option<f32>,
}

// ============================================================
// Tool Definition
// ============================================================

#[derive(Debug, Clone, PartialEq, StructuralConvert)]
#[convert(from(openai::FileSearchTool))]
pub struct FileSearchTool {
    pub vector_store_ids: Vec<String>,
    pub max_num_results: Option<u32>,
    pub filters: Option<Filter>,
    pub ranking_options: Option<RankingOptions>,
}

// ============================================================
// Output / Resource Supporting Types
// ============================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, StructuralConvert, Display)]
#[convert(from(openai::FileSearchToolCallStatus))]
#[strum(serialize_all = "snake_case")]
pub enum FileSearchToolCallStatus {
    InProgress,
    Searching,
    Incomplete,
    Failed,
    Completed,
}

#[derive(Debug, Clone, PartialEq, StructuralConvert)]
#[convert(from(openai::FileSearchToolCallResult))]
pub struct FileSearchToolCallResult {
    pub attributes: HashMap<String, Value>,
    pub file_id: String,
    pub filename: String,
    pub score: f32,
    pub text: String,
}

// ============================================================
// Output / Resource Shapes
// ============================================================

#[derive(Debug, Clone, PartialEq, StructuralConvert)]
#[convert(from(openai::FileSearchToolCall))]
pub struct FileSearchToolCall {
    pub id: String,
    pub queries: Vec<String>,
    pub status: FileSearchToolCallStatus,
    pub results: Option<Vec<FileSearchToolCallResult>>,
}
