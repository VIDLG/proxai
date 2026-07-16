use crate::protocol::openai::PromptCacheBreakpointConfig;
use crate::protocol::{OptionalNullable, deserialize_present};
use serde::{Deserialize, Serialize};

use strum::{AsRefStr, Display};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Display, Default, Serialize, Deserialize)]
#[strum(serialize_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum ImageDetail {
    #[default]
    Auto,
    Low,
    High,
    Original,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Display, Serialize, Deserialize)]
#[strum(serialize_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum FileInputDetail {
    Auto,
    Low,
    High,
}

/// OpenAPI schema: `#/components/schemas/InputTextContent`
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InputTextContent {
    pub text: String,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_present"
    )]
    pub prompt_cache_breakpoint: Option<PromptCacheBreakpointConfig>,
}

/// OpenAPI schema: `#/components/schemas/InputImageContent`
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct InputImageContent {
    #[serde(default, skip_serializing_if = "OptionalNullable::is_missing")]
    pub image_url: OptionalNullable<String>,
    #[serde(default, skip_serializing_if = "OptionalNullable::is_missing")]
    pub file_id: OptionalNullable<String>,
    #[serde(default)]
    pub detail: ImageDetail,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_present"
    )]
    pub prompt_cache_breakpoint: Option<PromptCacheBreakpointConfig>,
}

/// OpenAPI schema: `#/components/schemas/InputFileContent`
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InputFileContent {
    #[serde(default, skip_serializing_if = "OptionalNullable::is_missing")]
    pub file_id: OptionalNullable<String>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_present"
    )]
    pub filename: Option<String>,

    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_present"
    )]
    pub file_data: Option<String>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_present"
    )]
    pub prompt_cache_breakpoint: Option<PromptCacheBreakpointConfig>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_present"
    )]
    pub file_url: Option<String>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_present"
    )]
    pub detail: Option<FileInputDetail>,
}

#[allow(
    clippy::enum_variant_names,
    reason = "Mirrors OpenAI Responses input content variant names."
)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, AsRefStr)]
#[serde(tag = "type", rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum InputContent {
    InputText(InputTextContent),
    InputImage(InputImageContent),
    InputFile(InputFileContent),
}
