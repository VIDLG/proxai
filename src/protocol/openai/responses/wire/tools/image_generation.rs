use async_openai::types::responses as openai;
use serde::{Deserialize, Serialize};
use structural_convert::StructuralConvert;
use strum::Display;

// ============================================================
// Tool Definition Supporting Types
// ============================================================

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, StructuralConvert, Display, Default, Serialize, Deserialize,
)]
#[convert(from(openai::ImageGenToolBackground))]
#[strum(serialize_all = "lowercase")]
pub enum ImageGenToolBackground {
    Transparent,
    Opaque,
    #[default]
    Auto,
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, StructuralConvert, Display, Default, Serialize, Deserialize,
)]
#[convert(from(openai::InputFidelity))]
#[strum(serialize_all = "lowercase")]
pub enum InputFidelity {
    High,
    #[default]
    Low,
}

#[derive(Debug, Clone, PartialEq, Eq, StructuralConvert, Default, Serialize, Deserialize)]
#[convert(from(openai::ImageGenToolInputImageMask))]
pub struct ImageGenToolInputImageMask {
    pub image_url: Option<String>,
    pub file_id: Option<String>,
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, StructuralConvert, Display, Default, Serialize, Deserialize,
)]
#[convert(from(openai::ImageGenToolModeration))]
#[strum(serialize_all = "lowercase")]
pub enum ImageGenToolModeration {
    #[default]
    Auto,
    Low,
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, StructuralConvert, Display, Default, Serialize, Deserialize,
)]
#[convert(from(openai::ImageGenToolOutputFormat))]
#[strum(serialize_all = "lowercase")]
pub enum ImageGenToolOutputFormat {
    #[default]
    Png,
    Webp,
    Jpeg,
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, StructuralConvert, Display, Default, Serialize, Deserialize,
)]
#[convert(from(openai::ImageGenToolQuality))]
#[strum(serialize_all = "lowercase")]
pub enum ImageGenToolQuality {
    Low,
    Medium,
    High,
    #[default]
    Auto,
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, StructuralConvert, Display, Default, Serialize, Deserialize,
)]
#[convert(from(openai::ImageGenToolSize))]
pub enum ImageGenToolSize {
    #[default]
    Auto,
    #[strum(to_string = "1024x1024")]
    Size1024x1024,
    #[strum(to_string = "1024x1536")]
    Size1024x1536,
    #[strum(to_string = "1536x1024")]
    Size1536x1024,
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, StructuralConvert, Display, Default, Serialize, Deserialize,
)]
#[convert(from(openai::ImageGenActionEnum))]
#[strum(serialize_all = "lowercase")]
pub enum ImageGenActionEnum {
    Generate,
    Edit,
    #[default]
    Auto,
}

// ============================================================
// Tool Definition
// ============================================================

#[derive(Debug, Clone, PartialEq, Eq, StructuralConvert, Serialize, Deserialize)]
#[convert(from(openai::ImageGenTool))]
pub struct ImageGenTool {
    pub background: Option<ImageGenToolBackground>,
    pub input_fidelity: Option<InputFidelity>,
    pub input_image_mask: Option<ImageGenToolInputImageMask>,
    pub model: Option<String>,
    pub moderation: Option<ImageGenToolModeration>,
    pub output_compression: Option<u8>,
    pub output_format: Option<ImageGenToolOutputFormat>,
    pub partial_images: Option<u8>,
    pub quality: Option<ImageGenToolQuality>,
    pub size: Option<ImageGenToolSize>,
    pub action: Option<ImageGenActionEnum>,
}

// ============================================================
// Output / Resource Supporting Types
// ============================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, StructuralConvert, Display, Serialize, Deserialize)]
#[convert(from(openai::ImageGenToolCallStatus))]
#[strum(serialize_all = "snake_case")]
pub enum ImageGenToolCallStatus {
    InProgress,
    Completed,
    Generating,
    Failed,
}

// ============================================================
// Output / Resource Shapes
// ============================================================

#[derive(Debug, Clone, PartialEq, Eq, StructuralConvert, Serialize, Deserialize)]
#[convert(from(openai::ImageGenToolCall))]
pub struct ImageGenToolCall {
    pub id: String,
    pub result: Option<String>,
    pub status: ImageGenToolCallStatus,
}
