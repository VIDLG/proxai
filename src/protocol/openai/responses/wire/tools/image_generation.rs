use serde::{Deserialize, Serialize};
use strum::Display;

// ============================================================
// Tool Definition Supporting Types
// ============================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Display, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[strum(serialize_all = "lowercase")]
pub enum ImageGenToolBackground {
    Transparent,
    Opaque,
    #[default]
    Auto,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Display, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[strum(serialize_all = "lowercase")]
pub enum InputFidelity {
    High,
    #[default]
    Low,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct ImageGenToolInputImageMask {
    pub image_url: Option<String>,
    pub file_id: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Display, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[strum(serialize_all = "lowercase")]
pub enum ImageGenToolModeration {
    #[default]
    Auto,
    Low,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Display, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[strum(serialize_all = "lowercase")]
pub enum ImageGenToolOutputFormat {
    #[default]
    Png,
    Webp,
    Jpeg,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Display, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[strum(serialize_all = "lowercase")]
pub enum ImageGenToolQuality {
    Low,
    Medium,
    High,
    #[default]
    Auto,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum ImageGenToolSize {
    #[default]
    Auto,
    #[serde(rename = "1024x1024")]
    Size1024x1024,
    #[serde(rename = "1024x1536")]
    Size1024x1536,
    #[serde(rename = "1536x1024")]
    Size1536x1024,
    #[serde(untagged)]
    Other(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Display, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Display, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImageGenToolCall {
    pub id: String,
    pub result: Option<String>,
    pub status: ImageGenToolCallStatus,
}
