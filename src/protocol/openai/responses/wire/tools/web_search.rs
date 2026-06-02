use async_openai::types::responses as openai;
use serde::{Deserialize, Serialize};
use structural_convert::StructuralConvert;
use strum::Display;

// ============================================================
// Tool Definition Supporting Types
// ============================================================

#[derive(Debug, Clone, PartialEq, Eq, StructuralConvert, Serialize, Deserialize)]
#[convert(from(openai::WebSearchToolFilters))]
pub struct WebSearchToolFilters {
    pub allowed_domains: Option<Vec<String>>,
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, StructuralConvert, Display, Default, Serialize, Deserialize,
)]
#[convert(from(openai::WebSearchApproximateLocationType))]
#[serde(rename_all = "lowercase")]
#[strum(serialize_all = "lowercase")]
pub enum WebSearchApproximateLocationType {
    #[default]
    Approximate,
}

#[derive(Debug, Clone, PartialEq, Eq, StructuralConvert, Serialize, Deserialize)]
#[convert(from(openai::WebSearchApproximateLocation))]
pub struct WebSearchApproximateLocation {
    pub r#type: WebSearchApproximateLocationType,
    pub city: Option<String>,
    pub country: Option<String>,
    pub region: Option<String>,
    pub timezone: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, StructuralConvert, Display, Serialize, Deserialize)]
#[convert(from(openai::WebSearchToolSearchContextSize))]
#[serde(rename_all = "lowercase")]
#[strum(serialize_all = "lowercase")]
pub enum WebSearchToolSearchContextSize {
    Low,
    Medium,
    High,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, StructuralConvert, Display, Serialize, Deserialize)]
#[convert(from(openai::SearchContentType))]
#[serde(rename_all = "lowercase")]
#[strum(serialize_all = "lowercase")]
pub enum SearchContentType {
    Text,
    Image,
}

// ============================================================
// Tool Definition
// ============================================================

#[derive(Debug, Clone, PartialEq, Eq, StructuralConvert, Serialize, Deserialize)]
#[convert(from(openai::WebSearchTool))]
pub struct WebSearchTool {
    pub filters: Option<WebSearchToolFilters>,
    pub user_location: Option<WebSearchApproximateLocation>,
    pub search_context_size: Option<WebSearchToolSearchContextSize>,
    pub search_content_types: Option<Vec<SearchContentType>>,
}

// ============================================================
// Output / Resource Supporting Types
// ============================================================

#[derive(Debug, Clone, PartialEq, Eq, StructuralConvert, Serialize, Deserialize)]
#[convert(from(openai::WebSearchActionSearchSource))]
pub struct WebSearchActionSearchSource {
    pub r#type: String,
    pub url: String,
}

#[derive(Debug, Clone, PartialEq, Eq, StructuralConvert, Serialize, Deserialize)]
#[convert(from(openai::WebSearchActionSearch))]
pub struct WebSearchActionSearch {
    pub query: String,
    pub sources: Option<Vec<WebSearchActionSearchSource>>,
}

#[derive(Debug, Clone, PartialEq, Eq, StructuralConvert, Serialize, Deserialize)]
#[convert(from(openai::WebSearchActionOpenPage))]
pub struct WebSearchActionOpenPage {
    pub url: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, StructuralConvert, Serialize, Deserialize)]
#[convert(from(openai::WebSearchActionFind))]
pub struct WebSearchActionFind {
    pub url: String,
    pub pattern: String,
}

#[derive(Debug, Clone, PartialEq, Eq, StructuralConvert, Serialize, Deserialize)]
#[convert(from(openai::WebSearchToolCallAction))]
pub enum WebSearchToolCallAction {
    Search(WebSearchActionSearch),
    OpenPage(WebSearchActionOpenPage),
    Find(WebSearchActionFind),
    FindInPage(WebSearchActionFind),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, StructuralConvert, Display, Serialize, Deserialize)]
#[convert(from(openai::WebSearchToolCallStatus))]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum WebSearchToolCallStatus {
    InProgress,
    Searching,
    Completed,
    Failed,
}

// ============================================================
// Output / Resource Shapes
// ============================================================

#[derive(Debug, Clone, PartialEq, Eq, StructuralConvert, Serialize, Deserialize)]
#[convert(from(openai::WebSearchToolCall))]
pub struct WebSearchToolCall {
    pub action: WebSearchToolCallAction,
    pub id: String,
    pub status: WebSearchToolCallStatus,
}
