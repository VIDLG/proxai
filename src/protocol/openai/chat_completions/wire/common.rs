use async_openai::types::chat as openai;
use serde::{Deserialize, Serialize};
use structural_convert::StructuralConvert;
use strum::Display;

#[derive(Debug, Clone, Copy, PartialEq, Eq, StructuralConvert, Display, Serialize, Deserialize)]
#[convert(from(openai::ServiceTier))]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum ServiceTier {
    Auto,
    Default,
    Flex,
    Scale,
    Priority,
}
