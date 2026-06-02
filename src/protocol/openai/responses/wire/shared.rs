use async_openai::types::responses as openai;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use structural_convert::StructuralConvert;
use strum::Display;

#[derive(Debug, Clone, PartialEq, StructuralConvert, Serialize, Deserialize)]
#[convert(from(openai::Filter))]
pub enum Filter {
    Comparison(ComparisonFilter),
    Compound(CompoundFilter),
}

#[derive(Debug, Clone, PartialEq, StructuralConvert, Serialize, Deserialize)]
#[convert(from(openai::ComparisonFilter))]
pub struct ComparisonFilter {
    pub r#type: ComparisonType,
    pub key: String,
    pub value: Value,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, StructuralConvert, Display, Serialize, Deserialize)]
#[convert(from(openai::ComparisonType))]
pub enum ComparisonType {
    #[serde(rename = "eq")]
    #[strum(to_string = "eq")]
    Equals,
    #[serde(rename = "ne")]
    #[strum(to_string = "ne")]
    NotEquals,
    #[serde(rename = "gt")]
    #[strum(to_string = "gt")]
    GreaterThan,
    #[serde(rename = "gte")]
    #[strum(to_string = "gte")]
    GreaterThanOrEqual,
    #[serde(rename = "lt")]
    #[strum(to_string = "lt")]
    LessThan,
    #[serde(rename = "lte")]
    #[strum(to_string = "lte")]
    LessThanOrEqual,
    #[serde(rename = "in")]
    #[strum(to_string = "in")]
    In,
    #[serde(rename = "nin")]
    #[strum(to_string = "nin")]
    NotIn,
}

#[derive(Debug, Clone, PartialEq, StructuralConvert, Serialize, Deserialize)]
#[convert(from(openai::CompoundFilter))]
pub struct CompoundFilter {
    pub r#type: CompoundType,
    pub filters: Vec<Filter>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, StructuralConvert, Display, Serialize, Deserialize)]
#[convert(from(openai::CompoundType))]
#[serde(rename_all = "lowercase")]
#[strum(serialize_all = "lowercase")]
pub enum CompoundType {
    And,
    Or,
}
