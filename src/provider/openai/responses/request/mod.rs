//! OpenAI Responses provider-request preparation.

mod prepare;
mod projection;
mod summary;

pub(crate) const UPSTREAM_PATH: &str = "/v1/responses";

pub(crate) use self::prepare::{
    PreparedProviderRequest, prepare_provider_request, sanitize_provider_payload,
};
pub(crate) use self::summary::{RequestSummary, ToolCategory};

#[cfg(test)]
mod tests;
