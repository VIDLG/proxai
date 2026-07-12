use crate::protocol::{OptionalNullable, deserialize_present};
use serde::{Deserialize, Serialize};
use strum::Display;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Display, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[strum(serialize_all = "lowercase")]
pub enum WebSearchContextSize {
    Low,
    #[default]
    Medium,
    High,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Display, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[strum(serialize_all = "lowercase")]
pub enum WebSearchUserLocationType {
    Approximate,
}

/// OpenAPI schema: `#/components/schemas/WebSearchLocation`
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct WebSearchLocation {
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_present"
    )]
    pub country: Option<String>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_present"
    )]
    pub region: Option<String>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_present"
    )]
    pub city: Option<String>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_present"
    )]
    pub timezone: Option<String>,
}

/// OpenAPI schema:
/// `#/components/schemas/CreateChatCompletionRequest/allOf/1/properties/web_search_options/properties/user_location`
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WebSearchUserLocation {
    pub r#type: WebSearchUserLocationType,
    pub approximate: WebSearchLocation,
}

/// OpenAPI schema:
/// `#/components/schemas/CreateChatCompletionRequest/allOf/1/properties/web_search_options`
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct WebSearchOptions {
    #[serde(default, skip_serializing_if = "OptionalNullable::is_missing")]
    pub user_location: OptionalNullable<WebSearchUserLocation>,

    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_present"
    )]
    pub search_context_size: Option<WebSearchContextSize>,
}
