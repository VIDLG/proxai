use async_openai::types::chat as openai;
use structural_convert::StructuralConvert;
use strum::Display;

#[allow(
    dead_code,
    reason = "Retained for full request schema projection coverage."
)]
#[derive(Debug, Clone, PartialEq, Eq, StructuralConvert, Display)]
#[convert(from(openai::ChatCompletionAudioVoice))]
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, StructuralConvert, Display)]
#[convert(from(openai::ChatCompletionAudioFormat))]
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
#[derive(Debug, Clone, PartialEq, Eq, StructuralConvert)]
#[convert(from(openai::ChatCompletionAudio))]
pub struct ChatCompletionAudio {
    pub voice: ChatCompletionAudioVoice,
    pub format: ChatCompletionAudioFormat,
}
