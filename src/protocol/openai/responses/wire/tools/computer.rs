use async_openai::types::responses as openai;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use structural_convert::StructuralConvert;
use strum::Display;

use super::super::OutputStatus;

// ============================================================
// Tool Definition Supporting Types
// ============================================================

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, StructuralConvert, Display, Default, Deserialize, Serialize,
)]
#[convert(from(openai::ComputerEnvironment))]
#[serde(rename_all = "lowercase")]
#[strum(serialize_all = "lowercase")]
pub enum ComputerEnvironment {
    Windows,
    Mac,
    Linux,
    Ubuntu,
    #[default]
    Browser,
}

// ============================================================
// Tool Definition Shapes
// ============================================================

// Keep this local tool typed even though the upstream SDK fields are private.
// We cannot use `StructuralConvert` here because `openai::ComputerUsePreviewTool`
// does not expose its fields publicly, so we deserialize from its serialized JSON
// shape instead.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct ComputerUsePreviewTool {
    pub environment: ComputerEnvironment,
    pub display_width: u32,
    pub display_height: u32,
}

impl From<openai::ComputerUsePreviewTool> for ComputerUsePreviewTool {
    fn from(value: openai::ComputerUsePreviewTool) -> Self {
        serde_json::from_value(serde_json::to_value(value).unwrap_or(Value::Null))
            .expect("ComputerUsePreviewTool should match local protocol shape")
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default, StructuralConvert, Serialize, Deserialize)]
#[convert(from(openai::ComputerTool))]
pub struct ComputerTool {}

// ============================================================
// Output / Resource Supporting Types
// ============================================================

#[derive(Debug, Clone, PartialEq, Eq, StructuralConvert, Serialize, Deserialize)]
#[convert(from(openai::CoordParam))]
pub struct CoordParam {
    pub x: i32,
    pub y: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, StructuralConvert, Display, Serialize, Deserialize)]
#[convert(from(openai::ClickButtonType))]
#[serde(rename_all = "lowercase")]
#[strum(serialize_all = "lowercase")]
pub enum ClickButtonType {
    Left,
    Right,
    Wheel,
    Back,
    Forward,
}

#[derive(Debug, Clone, PartialEq, Eq, StructuralConvert, Serialize, Deserialize)]
#[convert(from(openai::ClickParam))]
pub struct ClickParam {
    pub button: ClickButtonType,
    pub x: i32,
    pub y: i32,
    pub keys: Option<Vec<String>>,
}

#[derive(Debug, Clone, PartialEq, Eq, StructuralConvert, Serialize, Deserialize)]
#[convert(from(openai::DoubleClickAction))]
pub struct DoubleClickAction {
    pub x: i32,
    pub y: i32,
    pub keys: Option<Vec<String>>,
}

#[derive(Debug, Clone, PartialEq, Eq, StructuralConvert, Serialize, Deserialize)]
#[convert(from(openai::DragParam))]
pub struct DragParam {
    pub path: Vec<CoordParam>,
    pub keys: Option<Vec<String>>,
}

#[derive(Debug, Clone, PartialEq, Eq, StructuralConvert, Serialize, Deserialize)]
#[convert(from(openai::KeyPressAction))]
pub struct KeyPressAction {
    pub keys: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, StructuralConvert, Serialize, Deserialize)]
#[convert(from(openai::MoveParam))]
pub struct MoveParam {
    pub x: i32,
    pub y: i32,
    pub keys: Option<Vec<String>>,
}

#[derive(Debug, Clone, PartialEq, Eq, StructuralConvert, Serialize, Deserialize)]
#[convert(from(openai::ScrollParam))]
pub struct ScrollParam {
    pub scroll_x: i32,
    pub scroll_y: i32,
    pub x: i32,
    pub y: i32,
    pub keys: Option<Vec<String>>,
}

#[derive(Debug, Clone, PartialEq, Eq, StructuralConvert, Serialize, Deserialize)]
#[convert(from(openai::TypeParam))]
pub struct TypeParam {
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Eq, StructuralConvert, Serialize, Deserialize)]
#[convert(from(openai::ComputerAction))]
pub enum ComputerAction {
    Click(ClickParam),
    DoubleClick(DoubleClickAction),
    Drag(DragParam),
    Keypress(KeyPressAction),
    Move(MoveParam),
    Screenshot,
    Scroll(ScrollParam),
    Type(TypeParam),
    Wait,
}

#[derive(Debug, Clone, PartialEq, Eq, StructuralConvert, Serialize, Deserialize)]
#[convert(from(openai::ComputerCallSafetyCheckParam))]
pub struct ComputerCallSafetyCheckParam {
    pub id: String,
    pub code: Option<String>,
    pub message: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, StructuralConvert, Display, Serialize, Deserialize)]
#[convert(from(openai::ComputerCallOutputStatus))]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum ComputerCallOutputStatus {
    InProgress,
    Completed,
    Incomplete,
    Failed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, StructuralConvert, Display, Serialize, Deserialize)]
#[convert(from(openai::ComputerScreenshotImageType))]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum ComputerScreenshotImageType {
    ComputerScreenshot,
}

#[derive(Debug, Clone, PartialEq, Eq, StructuralConvert, Serialize, Deserialize)]
#[convert(from(openai::ComputerScreenshotImage))]
pub struct ComputerScreenshotImage {
    pub r#type: ComputerScreenshotImageType,
    pub file_id: Option<String>,
    pub image_url: Option<String>,
}

// ============================================================
// Input / Context Item Shapes
// ============================================================

#[derive(Debug, Clone, PartialEq, Eq, StructuralConvert, Serialize, Deserialize)]
#[convert(from(openai::ComputerCallOutputItemParam))]
pub struct ComputerCallOutputItemParam {
    pub call_id: String,
    pub output: ComputerScreenshotImage,
    pub acknowledged_safety_checks: Option<Vec<ComputerCallSafetyCheckParam>>,
    pub id: Option<String>,
    pub status: Option<OutputStatus>,
}

// ============================================================
// Output / Resource Shapes
// ============================================================

#[derive(Debug, Clone, PartialEq, Eq, StructuralConvert, Serialize, Deserialize)]
#[convert(from(openai::ComputerToolCall))]
pub struct ComputerToolCall {
    pub action: Option<ComputerAction>,
    pub actions: Option<Vec<ComputerAction>>,
    pub call_id: String,
    pub id: String,
    pub pending_safety_checks: Vec<ComputerCallSafetyCheckParam>,
    pub status: OutputStatus,
}

#[derive(Debug, Clone, PartialEq, Eq, StructuralConvert, Serialize, Deserialize)]
#[convert(from(openai::ComputerToolCallOutputResource))]
pub struct ComputerToolCallOutputResource {
    pub call_id: String,
    pub output: ComputerScreenshotImage,
    pub acknowledged_safety_checks: Option<Vec<ComputerCallSafetyCheckParam>>,
    pub id: String,
    pub status: ComputerCallOutputStatus,
    pub created_by: Option<String>,
}
