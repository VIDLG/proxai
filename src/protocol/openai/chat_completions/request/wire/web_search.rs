use async_openai::types::chat as openai;
use structural_convert::StructuralConvert;
use strum::Display;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, StructuralConvert, Display)]
#[convert(from(openai::WebSearchContextSize))]
#[strum(serialize_all = "lowercase")]
pub enum WebSearchContextSize {
    Low,
    #[default]
    Medium,
    High,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, StructuralConvert, Display)]
#[convert(from(openai::WebSearchUserLocationType))]
#[strum(serialize_all = "lowercase")]
pub enum WebSearchUserLocationType {
    Approximate,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, StructuralConvert)]
#[convert(from(openai::WebSearchLocation))]
pub struct WebSearchLocation {
    pub country: Option<String>,
    pub region: Option<String>,
    pub city: Option<String>,
    pub timezone: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, StructuralConvert)]
#[convert(from(openai::WebSearchUserLocation))]
pub struct WebSearchUserLocation {
    pub r#type: WebSearchUserLocationType,
    pub approximate: WebSearchLocation,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, StructuralConvert)]
#[convert(from(openai::WebSearchOptions))]
pub struct WebSearchOptions {
    pub search_context_size: Option<WebSearchContextSize>,
    pub user_location: Option<WebSearchUserLocation>,
}
