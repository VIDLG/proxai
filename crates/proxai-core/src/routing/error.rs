use crate::protocol::RequestProtocol;

pub type Result<T> = std::result::Result<T, RoutingError>;

/// Failures while validating, compiling, or resolving model routing rules.
#[derive(Debug, thiserror::Error)]
pub enum RoutingError {
    #[error("routing.routes[{index}].name must be a non-empty string")]
    EmptyRouteName { index: usize },

    #[error("routing.routes[{index}].name duplicates route name `{name}`")]
    DuplicateRouteName { index: usize, name: String },

    #[error("routing.routes[{index}].model_pattern must be a non-empty string")]
    EmptyModelPattern { index: usize },

    #[error("provider name must be a non-empty string")]
    EmptyProviderName,

    #[error("provider name `{provider}` is configured more than once")]
    DuplicateProviderName { provider: String },

    #[error("routing.routes[{index}].provider must be a non-empty string")]
    EmptyRouteProvider { index: usize },

    #[error("routing.default_provider_names.{protocol} must be a non-empty string")]
    EmptyDefaultProvider { protocol: RequestProtocol },

    #[error("routing.default_provider_names.{protocol} references unknown provider `{provider}`")]
    UnknownDefaultProvider {
        protocol: RequestProtocol,
        provider: String,
    },

    #[error("routing.routes[{index}].provider references unknown provider `{provider}`")]
    UnknownRouteProvider { index: usize, provider: String },

    #[error("routing.routes[{index}].model_pattern `{pattern}` is not a valid glob: {source}")]
    InvalidGlob {
        index: usize,
        pattern: String,
        #[source]
        source: globset::Error,
    },

    #[error("routing.routes[{index}].model_pattern `{pattern}` is not a valid regex: {source}")]
    InvalidRegex {
        index: usize,
        pattern: String,
        #[source]
        source: regex::Error,
    },

    #[error(
        "routing route{} matches model `{model}` but request_protocol is `{configured}` while the inbound request uses `{inbound}`; remove request_protocol to accept any inbound protocol, or update it to `{inbound}`",
        route_name
            .as_deref()
            .map(|name| format!(" `{name}`"))
            .unwrap_or_default()
    )]
    RequestProtocolMismatch {
        route_name: Option<String>,
        model: String,
        configured: RequestProtocol,
        inbound: RequestProtocol,
    },
}
