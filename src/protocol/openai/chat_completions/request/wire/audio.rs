use async_openai::types::chat as openai;
use serde::{Deserialize, Serialize};
use structural_convert::StructuralConvert;
use strum::Display;

#[allow(
    dead_code,
    reason = "Retained for full request schema projection coverage."
)]
#[derive(Debug, Clone, PartialEq, Eq, StructuralConvert, Display, Serialize, Deserialize)]
#[convert(from(openai::ChatCompletionAudioVoice))]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum ChatCompletionAudioVoice {
    Alloy,
    Ash,
    Ballad,
    Coral,
    Echo,
    Fable,
    Nova,
    Onyx,
    Sage,
    Shimmer,
    #[convert(from(rename = "Other"))]
    Other(String),
}

#[allow(
    dead_code,
    reason = "Retained for full request schema projection coverage."
)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, StructuralConvert, Display, Serialize, Deserialize)]
#[convert(from(openai::ChatCompletionAudioFormat))]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum ChatCompletionAudioFormat {
    Wav,
    Aac,
    Mp3,
    Flac,
    Opus,
    Pcm16,
}

#[allow(
    dead_code,
    reason = "Retained for full request schema projection coverage."
)]
#[derive(Debug, Clone, PartialEq, Eq, StructuralConvert, Serialize, Deserialize)]
#[convert(from(openai::ChatCompletionAudio))]
pub struct ChatCompletionAudio {
    pub voice: ChatCompletionAudioVoice,
    pub format: ChatCompletionAudioFormat,
}
