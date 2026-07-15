use crate::protocol::{OptionalNullable, deserialize_present};
use serde::{Deserialize, Serialize};
use strum::Display;

// ============================================================
// Tool Definition Supporting Types
// ============================================================

/// OpenAPI schema: `#/components/schemas/WebSearchTool/properties/filters/anyOf/0`
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WebSearchToolFilters {
    #[serde(default, skip_serializing_if = "OptionalNullable::is_missing")]
    pub allowed_domains: OptionalNullable<Vec<String>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Display, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[strum(serialize_all = "lowercase")]
pub enum WebSearchApproximateLocationType {
    #[default]
    Approximate,
}

/// OpenAPI schema: `#/components/schemas/WebSearchApproximateLocation`
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WebSearchApproximateLocation {
    #[serde(default)]
    pub r#type: WebSearchApproximateLocationType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub city: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub country: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub region: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
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

// ============================================================
// Tool Definition
// ============================================================

/// OpenAPI schema: `#/components/schemas/WebSearchTool`
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WebSearchTool {
    #[serde(default, skip_serializing_if = "OptionalNullable::is_missing")]
    pub filters: OptionalNullable<WebSearchToolFilters>,
    #[serde(default, skip_serializing_if = "OptionalNullable::is_missing")]
    pub user_location: OptionalNullable<WebSearchApproximateLocation>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_present"
    )]
    pub search_context_size: Option<WebSearchToolSearchContextSize>,
}

/// OpenAPI schema: `#/components/schemas/ApproximateLocation`
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApproximateLocation {
    #[serde(default)]
    pub r#type: WebSearchApproximateLocationType,
    #[serde(default, skip_serializing_if = "OptionalNullable::is_missing")]
    pub country: OptionalNullable<String>,
    #[serde(default, skip_serializing_if = "OptionalNullable::is_missing")]
    pub region: OptionalNullable<String>,
    #[serde(default, skip_serializing_if = "OptionalNullable::is_missing")]
    pub city: OptionalNullable<String>,
    #[serde(default, skip_serializing_if = "OptionalNullable::is_missing")]
    pub timezone: OptionalNullable<String>,
}

/// OpenAPI schema: `#/components/schemas/SearchContextSize`
#[derive(Debug, Clone, Copy, PartialEq, Eq, Display, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[strum(serialize_all = "lowercase")]
pub enum SearchContextSize {
    Low,
    Medium,
    High,
}

/// OpenAPI schema: `#/components/schemas/SearchContentType`
#[derive(Debug, Clone, Copy, PartialEq, Eq, Display, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[strum(serialize_all = "lowercase")]
pub enum SearchContentType {
    Text,
    Image,
}

/// OpenAPI schema: `#/components/schemas/WebSearchPreviewTool`
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WebSearchPreviewTool {
    #[serde(default, skip_serializing_if = "OptionalNullable::is_missing")]
    pub user_location: OptionalNullable<ApproximateLocation>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_present"
    )]
    pub search_context_size: Option<SearchContextSize>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_present"
    )]
    pub search_content_types: Option<Vec<SearchContentType>>,
}

// ============================================================
// Output / Resource Supporting Types
// ============================================================

/// OpenAPI schema:
/// `#/components/schemas/WebSearchActionSearch/properties/sources/items/properties/type`
#[derive(Debug, Clone, Copy, PartialEq, Eq, Display, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum WebSearchActionSearchSourceType {
    Url,
}

/// OpenAPI schema: `#/components/schemas/WebSearchActionSearch/properties/sources/items`
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WebSearchActionSearchSource {
    pub r#type: WebSearchActionSearchSourceType,
    pub url: String,
}

/// OpenAPI schema: `#/components/schemas/WebSearchActionSearch`
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WebSearchActionSearch {
    pub query: String,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_present"
    )]
    pub queries: Option<Vec<String>>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_present"
    )]
    pub sources: Option<Vec<WebSearchActionSearchSource>>,
}

/// OpenAPI schema: `#/components/schemas/WebSearchActionOpenPage`
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WebSearchActionOpenPage {
    #[serde(default, skip_serializing_if = "OptionalNullable::is_missing")]
    pub url: OptionalNullable<String>,
}

/// OpenAPI schema: `#/components/schemas/WebSearchActionFind`
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WebSearchActionFind {
    pub url: String,
    pub pattern: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum WebSearchToolCallAction {
    Search(WebSearchActionSearch),
    OpenPage(WebSearchActionOpenPage),
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

/// OpenAPI schema: `#/components/schemas/WebSearchToolCall`
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WebSearchToolCall {
    pub id: String,
    pub status: WebSearchToolCallStatus,

    pub action: WebSearchToolCallAction,
}
