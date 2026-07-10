use serde::{Deserialize, Serialize};
use serde_json::Value;
use strum::Display;

use std::collections::HashMap;

use super::super::Filter;

// ============================================================
// Tool Definition Supporting Types
// ============================================================

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HybridSearch {
    pub embedding_weight: f32,
    pub text_weight: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Display, Serialize, Deserialize)]
pub enum RankVersionType {
    #[serde(rename = "auto")]
    Auto,
    #[serde(rename = "default-2024-11-15")]
    #[strum(to_string = "default-2024-11-15")]
    Default20241115,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RankingOptions {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hybrid_search: Option<HybridSearch>,
    pub ranker: RankVersionType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub score_threshold: Option<f32>,
}

// ============================================================
// Tool Definition
// ============================================================

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FileSearchTool {
    pub vector_store_ids: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_num_results: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filters: Option<Filter>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ranking_options: Option<RankingOptions>,
}

// ============================================================
// Output / Resource Supporting Types
// ============================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Display, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum FileSearchToolCallStatus {
    InProgress,
    Searching,
    Incomplete,
    Failed,
    Completed,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FileSearchToolCall {
    pub id: String,
    pub queries: Vec<String>,
    pub status: FileSearchToolCallStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub results: Option<Vec<FileSearchToolCallResult>>,
}
