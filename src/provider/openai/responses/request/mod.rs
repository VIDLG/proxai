//! OpenAI Responses forwarded-request preparation.

mod prepare;
mod projection;
mod summary;

pub(crate) const UPSTREAM_PATH: &str = "/v1/responses";

pub(crate) use self::prepare::{prepare_forwarded_request, PreparedForwardedRequest};
pub(crate) use self::summary::{RequestSummary, ToolCategory};

#[cfg(test)]
mod tests;
