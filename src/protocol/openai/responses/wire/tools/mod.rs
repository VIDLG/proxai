use async_openai::types::responses as openai;
use structural_convert::StructuralConvert;

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

#[allow(
    clippy::enum_variant_names,
    reason = "Mirrors OpenAI Responses Tool variant names."
)]
#[derive(Debug, Clone, PartialEq, StructuralConvert)]
#[convert(from(openai::Tool))]
pub enum Tool {
    Function(FunctionTool),
    FileSearch(FileSearchTool),
    ComputerUsePreview(ComputerUsePreviewTool),
    WebSearch(WebSearchTool),
    WebSearch20250826(WebSearchTool),
    Mcp(MCPTool),
    CodeInterpreter(CodeInterpreterTool),
    ImageGeneration(ImageGenTool),
    LocalShell,
    Shell(FunctionShellToolParam),
    Custom(CustomToolParam),
    Computer(ComputerTool),
    Namespace(NamespaceToolParam),
    ToolSearch(ToolSearchToolParam),
    WebSearchPreview(WebSearchTool),
    WebSearchPreview20250311(WebSearchTool),
    ApplyPatch,
}
