use async_openai::types::responses as openai;
use serde_json::Value;
use structural_convert::StructuralConvert;
use strum::Display;

#[derive(Debug, Clone, PartialEq, StructuralConvert)]
#[convert(from(openai::Filter))]
pub enum Filter {
    Comparison(ComparisonFilter),
    Compound(CompoundFilter),
}

#[derive(Debug, Clone, PartialEq, StructuralConvert)]
#[convert(from(openai::ComparisonFilter))]
pub struct ComparisonFilter {
    pub r#type: ComparisonType,
    pub key: String,
    pub value: Value,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, StructuralConvert, Display)]
#[convert(from(openai::ComparisonType))]
pub enum ComparisonType {
    #[strum(to_string = "eq")]
    Equals,
    #[strum(to_string = "ne")]
    NotEquals,
    #[strum(to_string = "gt")]
    GreaterThan,
    #[strum(to_string = "gte")]
    GreaterThanOrEqual,
    #[strum(to_string = "lt")]
    LessThan,
    #[strum(to_string = "lte")]
    LessThanOrEqual,
    #[strum(to_string = "in")]
    In,
    #[strum(to_string = "nin")]
    NotIn,
}

#[derive(Debug, Clone, PartialEq, StructuralConvert)]
#[convert(from(openai::CompoundFilter))]
pub struct CompoundFilter {
    pub r#type: CompoundType,
    pub filters: Vec<Filter>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, StructuralConvert, Display)]
#[convert(from(openai::CompoundType))]
#[strum(serialize_all = "lowercase")]
pub enum CompoundType {
    And,
    Or,
}
