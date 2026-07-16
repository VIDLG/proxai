use serde::{Deserialize, Serialize};

pub mod chat_completions;
pub mod responses;

/// OpenAPI schema: `#/components/schemas/PromptCacheBreakpointParam`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PromptCacheBreakpointParam {
    pub mode: PromptCacheBreakpointMode,
}

/// OpenAPI schema: `#/components/schemas/PromptCacheBreakpointConfig`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PromptCacheBreakpointConfig {
    pub mode: PromptCacheBreakpointMode,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PromptCacheBreakpointMode {
    Explicit,
}

#[cfg(test)]
#[path = "prompt_cache_tests.rs"]
mod tests;
