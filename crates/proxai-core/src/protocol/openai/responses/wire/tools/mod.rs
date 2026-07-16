use serde::{Deserialize, Serialize};
use strum::AsRefStr;

mod apply_patch;
mod code_interpreter;
mod computer;
mod custom;
mod file_search;
mod function;
mod function_shell;
mod image_generation;
mod local_shell;
mod mcp;
mod namespace;
mod tool_search;
mod web_search;

#[allow(unused_imports, reason = "Family facade re-exports.")]
pub use self::apply_patch::*;
#[allow(unused_imports, reason = "Family facade re-exports.")]
pub use self::code_interpreter::*;
#[allow(unused_imports, reason = "Family facade re-exports.")]
pub use self::computer::*;
#[allow(unused_imports, reason = "Family facade re-exports.")]
pub use self::custom::*;
#[allow(unused_imports, reason = "Family facade re-exports.")]
pub use self::file_search::*;
#[allow(unused_imports, reason = "Family facade re-exports.")]
pub use self::function::*;
#[allow(unused_imports, reason = "Family facade re-exports.")]
pub use self::function_shell::*;
#[allow(unused_imports, reason = "Family facade re-exports.")]
pub use self::image_generation::*;
#[allow(unused_imports, reason = "Family facade re-exports.")]
pub use self::local_shell::*;
#[allow(unused_imports, reason = "Family facade re-exports.")]
pub use self::mcp::*;
#[allow(unused_imports, reason = "Family facade re-exports.")]
pub use self::namespace::*;
#[allow(unused_imports, reason = "Family facade re-exports.")]
pub use self::tool_search::*;
#[allow(unused_imports, reason = "Family facade re-exports.")]
pub use self::web_search::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CallableToolAllowedCaller {
    Direct,
    Programmatic,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ToolCallCaller {
    Direct,
    Program { caller_id: String },
}

pub type ToolCallCallerParam = ToolCallCaller;

#[allow(
    clippy::enum_variant_names,
    reason = "Mirrors OpenAI Responses Tool variant names."
)]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, AsRefStr)]
#[serde(tag = "type", rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum Tool {
    Function(FunctionTool),
    FileSearch(FileSearchTool),
    ComputerUsePreview(ComputerUsePreviewTool),
    WebSearch(WebSearchTool),
    #[serde(rename = "web_search_2025_08_26")]
    #[strum(serialize = "web_search_2025_08_26")]
    WebSearch20250826(WebSearchTool),
    Mcp(MCPTool),
    CodeInterpreter(CodeInterpreterTool),
    ImageGeneration(ImageGenTool),
    LocalShell,
    Shell(FunctionShellToolParam),
    Custom(CustomToolParam),
    Computer(ComputerTool),
    Namespace(NamespaceToolParam),
    ProgrammaticToolCalling,
    ToolSearch(ToolSearchToolParam),
    WebSearchPreview(WebSearchPreviewTool),
    #[serde(rename = "web_search_preview_2025_03_11")]
    #[strum(serialize = "web_search_preview_2025_03_11")]
    WebSearchPreview20250311(WebSearchPreviewTool),
    ApplyPatch,
}
