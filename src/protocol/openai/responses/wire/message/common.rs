use async_openai::types::responses as openai;
use serde::{Deserialize, Serialize};
use structural_convert::StructuralConvert;
use strum::Display;

#[derive(Debug, Clone, Copy, PartialEq, Eq, StructuralConvert, Display, Serialize, Deserialize)]
#[convert(from(openai::MessagePhase))]
#[strum(serialize_all = "snake_case")]
pub enum MessagePhase {
    Commentary,
    FinalAnswer,
}
