use serde::{Deserialize, Serialize};
use strum::Display;

/// OpenAPI schema: `#/components/schemas/VoiceIdsOrCustomVoice`
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum VoiceIdsOrCustomVoice {
    BuiltIn(String),
    Custom { id: String },
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

/// OpenAPI schema: `#/components/schemas/CreateChatCompletionRequest/allOf/1/properties/audio`
#[allow(
    dead_code,
    reason = "Retained for full request schema projection coverage."
)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChatCompletionAudio {
    pub voice: VoiceIdsOrCustomVoice,
    pub format: ChatCompletionAudioFormat,
}

#[cfg(test)]
#[path = "audio_tests.rs"]
mod tests;
