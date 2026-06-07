use serde::{Deserialize, Serialize};
use strum::Display;

#[allow(
    dead_code,
    reason = "Retained for full request schema projection coverage."
)]
#[derive(Debug, Clone, PartialEq, Eq, Display, Serialize, Deserialize)]
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
    Other(String),
}

#[allow(
    dead_code,
    reason = "Retained for full request schema projection coverage."
)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Display, Serialize, Deserialize)]
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
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChatCompletionAudio {
    pub voice: ChatCompletionAudioVoice,
    pub format: ChatCompletionAudioFormat,
}
