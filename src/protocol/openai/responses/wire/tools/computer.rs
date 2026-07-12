use crate::protocol::RequiredNullable;
use crate::protocol::{OptionalNullable, deserialize_present};
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

/// OpenAPI schema: `#/components/schemas/ComputerUsePreviewTool`
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct ComputerUsePreviewTool {
    pub environment: ComputerEnvironment,
    pub display_width: u32,
    pub display_height: u32,
}

/// OpenAPI schema: `#/components/schemas/ComputerTool`
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct ComputerTool {}

// ============================================================
// Output / Resource Supporting Types
// ============================================================

/// OpenAPI schema: `#/components/schemas/CoordParam`
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

/// OpenAPI schema: `#/components/schemas/ClickParam`
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClickParam {
    pub button: ClickButtonType,
    pub x: i32,
    pub y: i32,
    #[serde(default, skip_serializing_if = "OptionalNullable::is_missing")]
    pub keys: OptionalNullable<Vec<String>>,
}

/// OpenAPI schema: `#/components/schemas/DoubleClickAction`
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DoubleClickAction {
    pub x: i32,
    pub y: i32,
    pub keys: RequiredNullable<Vec<String>>,
}

/// OpenAPI schema: `#/components/schemas/DragParam`
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DragParam {
    pub path: Vec<CoordParam>,
    #[serde(default, skip_serializing_if = "OptionalNullable::is_missing")]
    pub keys: OptionalNullable<Vec<String>>,
}

/// OpenAPI schema: `#/components/schemas/KeyPressAction`
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KeyPressAction {
    pub keys: Vec<String>,
}

/// OpenAPI schema: `#/components/schemas/MoveParam`
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MoveParam {
    pub x: i32,
    pub y: i32,
    #[serde(default, skip_serializing_if = "OptionalNullable::is_missing")]
    pub keys: OptionalNullable<Vec<String>>,
}

/// OpenAPI schema: `#/components/schemas/ScrollParam`
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScrollParam {
    pub x: i32,
    pub y: i32,

    pub scroll_x: i32,
    pub scroll_y: i32,
    #[serde(default, skip_serializing_if = "OptionalNullable::is_missing")]
    pub keys: OptionalNullable<Vec<String>>,
}

/// OpenAPI schema: `#/components/schemas/TypeParam`
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TypeParam {
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
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

/// OpenAPI schema: `#/components/schemas/ComputerCallSafetyCheckParam`
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ComputerCallSafetyCheckParam {
    pub id: String,
    #[serde(default, skip_serializing_if = "OptionalNullable::is_missing")]
    pub code: OptionalNullable<String>,
    #[serde(default, skip_serializing_if = "OptionalNullable::is_missing")]
    pub message: OptionalNullable<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Display, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum ComputerCallOutputStatus {
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

/// OpenAPI schema: `#/components/schemas/ComputerScreenshotImage`
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ComputerScreenshotImage {
    pub r#type: ComputerScreenshotImageType,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_present"
    )]
    pub image_url: Option<String>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_present"
    )]
    pub file_id: Option<String>,
}

// ============================================================
// Input / Context Item Shapes
// ============================================================

/// OpenAPI schema: `#/components/schemas/ComputerCallOutputItemParam`
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ComputerCallOutputItemParam {
    #[serde(default, skip_serializing_if = "OptionalNullable::is_missing")]
    pub id: OptionalNullable<String>,

    pub call_id: String,
    pub output: ComputerScreenshotImage,
    #[serde(default, skip_serializing_if = "OptionalNullable::is_missing")]
    pub acknowledged_safety_checks: OptionalNullable<Vec<ComputerCallSafetyCheckParam>>,
    #[serde(default, skip_serializing_if = "OptionalNullable::is_missing")]
    pub status: OptionalNullable<OutputStatus>,
}

// ============================================================
// Output / Resource Shapes
// ============================================================

/// OpenAPI schema: `#/components/schemas/ComputerToolCall`
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ComputerToolCall {
    pub id: String,
    pub call_id: String,

    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_present"
    )]
    pub action: Option<ComputerAction>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_present"
    )]
    pub actions: Option<Vec<ComputerAction>>,
    pub pending_safety_checks: Vec<ComputerCallSafetyCheckParam>,
    pub status: OutputStatus,
}

/// OpenAPI schema: `#/components/schemas/ComputerToolCallOutputResource`
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ComputerToolCallOutputResource {
    pub id: String,

    pub call_id: String,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_present"
    )]
    pub acknowledged_safety_checks: Option<Vec<ComputerCallSafetyCheckParam>>,
    pub output: ComputerScreenshotImage,
    pub status: ComputerCallOutputStatus,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_present"
    )]
    pub created_by: Option<String>,
}
