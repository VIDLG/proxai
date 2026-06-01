use async_openai::types::responses as openai;
use structural_convert::StructuralConvert;
use strum::Display;

use super::super::{InputContent, OutputStatus};
use super::MessagePhase;

#[derive(Debug, Clone, Copy, PartialEq, Eq, StructuralConvert, Display, Default)]
#[convert(from(openai::InputRole))]
#[strum(serialize_all = "lowercase")]
pub enum InputRole {
    #[default]
    User,
    System,
    Developer,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, StructuralConvert, Display, Default)]
#[convert(from(openai::MessageType))]
#[strum(serialize_all = "lowercase")]
pub enum MessageType {
    #[default]
    Message,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, StructuralConvert, Display, Default)]
#[convert(from(openai::Role))]
#[strum(serialize_all = "lowercase")]
pub enum Role {
    #[default]
    User,
    Assistant,
    System,
    Developer,
}

#[derive(Debug, Clone, PartialEq, Eq, StructuralConvert)]
#[convert(from(openai::EasyInputContent))]
pub enum EasyInputContent {
    Text(String),
    ContentList(Vec<InputContent>),
}

#[derive(Debug, Clone, PartialEq, Eq, StructuralConvert)]
#[convert(from(openai::EasyInputMessage))]
pub struct EasyInputMessage {
    pub r#type: MessageType,
    pub role: Role,
    pub content: EasyInputContent,
    pub phase: Option<MessagePhase>,
}

#[derive(Debug, Clone, PartialEq, Eq, StructuralConvert)]
#[convert(from(openai::InputMessage))]
pub struct InputMessage {
    pub content: Vec<InputContent>,
    pub role: InputRole,
    pub status: Option<OutputStatus>,
}
