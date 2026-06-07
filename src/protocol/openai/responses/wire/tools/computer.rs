use serde::{Deserialize, Serialize};
use strum::Display;

use super::super::OutputStatus;

// ============================================================
// Tool Definition Supporting Types
// ============================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Display, Default, Deserialize, Serialize)]
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

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct ComputerUsePreviewTool {
    pub environment: ComputerEnvironment,
    pub display_width: u32,
    pub display_height: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct ComputerTool {}

// ============================================================
// Output / Resource Supporting Types
// ============================================================

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CoordParam {
    pub x: i32,
    pub y: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Display, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[strum(serialize_all = "lowercase")]
pub enum ClickButtonType {
    Left,
    Right,
    Wheel,
    Back,
    Forward,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClickParam {
    pub button: ClickButtonType,
    pub x: i32,
    pub y: i32,
    pub keys: Option<Vec<String>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DoubleClickAction {
    pub x: i32,
    pub y: i32,
    pub keys: Option<Vec<String>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DragParam {
    pub path: Vec<CoordParam>,
    pub keys: Option<Vec<String>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KeyPressAction {
    pub keys: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MoveParam {
    pub x: i32,
    pub y: i32,
    pub keys: Option<Vec<String>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScrollParam {
    pub scroll_x: i32,
    pub scroll_y: i32,
    pub x: i32,
    pub y: i32,
    pub keys: Option<Vec<String>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TypeParam {
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ComputerCallSafetyCheckParam {
    pub id: String,
    pub code: Option<String>,
    pub message: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Display, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum ComputerCallOutputStatus {
    InProgress,
    Completed,
    Incomplete,
    Failed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Display, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum ComputerScreenshotImageType {
    ComputerScreenshot,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ComputerScreenshotImage {
    pub r#type: ComputerScreenshotImageType,
    pub file_id: Option<String>,
    pub image_url: Option<String>,
}

// ============================================================
// Input / Context Item Shapes
// ============================================================

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ComputerToolCall {
    pub action: Option<ComputerAction>,
    pub actions: Option<Vec<ComputerAction>>,
    pub call_id: String,
    pub id: String,
    pub pending_safety_checks: Vec<ComputerCallSafetyCheckParam>,
    pub status: OutputStatus,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ComputerToolCallOutputResource {
    pub call_id: String,
    pub output: ComputerScreenshotImage,
    pub acknowledged_safety_checks: Option<Vec<ComputerCallSafetyCheckParam>>,
    pub id: String,
    pub status: ComputerCallOutputStatus,
    pub created_by: Option<String>,
}
