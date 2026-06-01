use async_openai::types::chat as openai;
use structural_convert::StructuralConvert;
use strum::Display;

#[derive(Debug, Clone, Copy, PartialEq, Eq, StructuralConvert, Display)]
#[convert(from(openai::ServiceTier))]
#[strum(serialize_all = "snake_case")]
pub enum ServiceTier {
    Auto,
    Default,
    Flex,
    Scale,
    Priority,
}
