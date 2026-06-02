//! OpenAI Chat Completions forwarded-request preparation.

mod prepare;
mod summary;

pub(crate) const UPSTREAM_PATH: &str = "/v1/chat/completions";

pub(crate) use self::prepare::{prepare_forwarded_request, PreparedForwardedRequest};
pub(crate) use self::summary::{RequestSummary, ToolCategory};

#[cfg(test)]
mod tests;
