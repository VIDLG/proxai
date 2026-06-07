use serde::{Deserialize, Serialize};
use strum::Display;

// ============================================================
// Tool Definition Supporting Types
// ============================================================

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WebSearchToolFilters {
    pub allowed_domains: Option<Vec<String>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Display, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[strum(serialize_all = "lowercase")]
pub enum WebSearchApproximateLocationType {
    #[default]
    Approximate,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WebSearchApproximateLocation {
    pub r#type: WebSearchApproximateLocationType,
    pub city: Option<String>,
    pub country: Option<String>,
    pub region: Option<String>,
    pub timezone: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Display, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[strum(serialize_all = "lowercase")]
pub enum WebSearchToolSearchContextSize {
    Low,
    Medium,
    High,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Display, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[strum(serialize_all = "lowercase")]
pub enum SearchContentType {
    Text,
    Image,
}

// ============================================================
// Tool Definition
// ============================================================

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WebSearchTool {
    pub filters: Option<WebSearchToolFilters>,
    pub user_location: Option<WebSearchApproximateLocation>,
    pub search_context_size: Option<WebSearchToolSearchContextSize>,
    pub search_content_types: Option<Vec<SearchContentType>>,
}

// ============================================================
// Output / Resource Supporting Types
// ============================================================

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WebSearchActionSearchSource {
    pub r#type: String,
    pub url: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WebSearchActionSearch {
    pub query: String,
    pub sources: Option<Vec<WebSearchActionSearchSource>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WebSearchActionOpenPage {
    pub url: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WebSearchActionFind {
    pub url: String,
    pub pattern: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum WebSearchToolCallAction {
    Search(WebSearchActionSearch),
    OpenPage(WebSearchActionOpenPage),
    Find(WebSearchActionFind),
    FindInPage(WebSearchActionFind),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Display, Serialize, Deserialize)]
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WebSearchToolCall {
    pub action: WebSearchToolCallAction,
    pub id: String,
    pub status: WebSearchToolCallStatus,
}
