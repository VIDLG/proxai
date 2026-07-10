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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EasyInputMessage {
    #[serde(default)]
    pub r#type: MessageType,
    pub role: Role,
    pub content: EasyInputContent,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub phase: Option<MessagePhase>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InputMessage {
    pub content: Vec<InputContent>,
    pub role: InputRole,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<OutputStatus>,
}
