use crate::protocol::{OptionalNullable, deserialize_present};
use serde::{Deserialize, Serialize};
use strum::Display;

use super::super::{InputContent, OutputStatus};
use super::MessagePhase;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Display, Default, Serialize, Deserialize)]
#[strum(serialize_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum InputRole {
    #[default]
    User,
    System,
    Developer,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Display, Default, Serialize, Deserialize)]
#[strum(serialize_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum MessageType {
    #[default]
    Message,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Display, Default, Serialize, Deserialize)]
#[strum(serialize_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum Role {
    #[default]
    User,
    Assistant,
    System,
    Developer,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum EasyInputContent {
    Text(String),
    ContentList(Vec<InputContent>),
}

/// OpenAPI schema: `#/components/schemas/EasyInputMessage`
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EasyInputMessage {
    pub role: Role,
    pub content: EasyInputContent,
    #[serde(default, skip_serializing_if = "OptionalNullable::is_missing")]
    pub phase: OptionalNullable<MessagePhase>,

    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_present"
    )]
    pub r#type: Option<MessageType>,
}

/// OpenAPI schema: `#/components/schemas/InputMessage`
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InputMessage {
    pub role: InputRole,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_present"
    )]
    pub status: Option<OutputStatus>,

    pub content: Vec<InputContent>,
}
