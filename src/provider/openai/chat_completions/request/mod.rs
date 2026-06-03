//! OpenAI Chat Completions provider-request preparation.

mod prepare;
mod summary;

pub(crate) const UPSTREAM_PATH: &str = "/v1/chat/completions";

pub(crate) use self::prepare::{PreparedProviderRequest, prepare_provider_request};
pub(crate) use self::summary::{RequestSummary, ToolCategory};

#[cfg(test)]
mod tests;
