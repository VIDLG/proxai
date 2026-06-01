use async_openai::types::responses as openai;
use serde::{Deserialize, Serialize};
use structural_convert::StructuralConvert;
use strum::Display;

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, StructuralConvert, Display, Default, Serialize, Deserialize,
)]
#[convert(from(openai::ImageDetail))]
#[strum(serialize_all = "lowercase")]
pub enum ImageDetail {
    #[default]
    Auto,
    Low,
    High,
    Original,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, StructuralConvert, Display, Serialize, Deserialize)]
#[convert(from(openai::FileInputDetail))]
#[strum(serialize_all = "lowercase")]
pub enum FileInputDetail {
    Low,
    High,
}

#[derive(Debug, Clone, PartialEq, Eq, StructuralConvert, Serialize, Deserialize)]
#[convert(from(openai::InputTextContent))]
pub struct InputTextContent {
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Eq, StructuralConvert, Serialize, Deserialize)]
#[convert(from(openai::InputImageContent))]
pub struct InputImageContent {
    pub detail: ImageDetail,
    pub file_id: Option<String>,
    pub image_url: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, StructuralConvert, Serialize, Deserialize)]
#[convert(from(openai::InputFileContent))]
pub struct InputFileContent {
    pub file_data: Option<String>,
    pub file_id: Option<String>,
    pub file_url: Option<String>,
    pub filename: Option<String>,
    pub detail: Option<FileInputDetail>,
}

#[allow(
    clippy::enum_variant_names,
    reason = "Mirrors OpenAI Responses input content variant names."
)]
#[derive(Debug, Clone, PartialEq, Eq, StructuralConvert, Serialize, Deserialize)]
#[convert(from(openai::InputContent))]
pub enum InputContent {
    InputText(InputTextContent),
    InputImage(InputImageContent),
    InputFile(InputFileContent),
}
