use crate::protocol::{OptionalNullable, deserialize_present};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use strum::Display;

use std::collections::HashMap;

use super::super::Filters;

// ============================================================
// Tool Definition Supporting Types
// ============================================================

/// OpenAPI schema: `#/components/schemas/HybridSearchOptions`
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HybridSearchOptions {
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

/// OpenAPI schema: `#/components/schemas/RankingOptions`
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RankingOptions {
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_present"
    )]
    pub ranker: Option<RankVersionType>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_present"
    )]
    pub score_threshold: Option<f32>,

    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_present"
    )]
    pub hybrid_search: Option<HybridSearchOptions>,
}

// ============================================================
// Tool Definition
// ============================================================

/// OpenAPI schema: `#/components/schemas/FileSearchTool`
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FileSearchTool {
    pub vector_store_ids: Vec<String>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_present"
    )]
    pub max_num_results: Option<u32>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_present"
    )]
    pub ranking_options: Option<RankingOptions>,
    #[serde(default, skip_serializing_if = "OptionalNullable::is_missing")]
    pub filters: OptionalNullable<Filters>,
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

/// OpenAPI schema: `#/components/schemas/FileSearchToolCall/properties/results/anyOf/0/items`
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FileSearchToolCallResult {
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_present"
    )]
    pub file_id: Option<String>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_present"
    )]
    pub text: Option<String>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_present"
    )]
    pub filename: Option<String>,

    #[serde(default, skip_serializing_if = "OptionalNullable::is_missing")]
    pub attributes: OptionalNullable<HashMap<String, Value>>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_present"
    )]
    pub score: Option<f32>,
}

// ============================================================
// Output / Resource Shapes
// ============================================================

/// OpenAPI schema: `#/components/schemas/FileSearchToolCall`
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FileSearchToolCall {
    pub id: String,
    pub status: FileSearchToolCallStatus,
    pub queries: Vec<String>,
    #[serde(default, skip_serializing_if = "OptionalNullable::is_missing")]
    pub results: OptionalNullable<Vec<FileSearchToolCallResult>>,
}
