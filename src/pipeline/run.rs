use axum::body::Body;
use axum::http::Response;

use crate::error::Result;
use crate::provider::ProviderTransport;

use super::provider_request::PreparedProviderFlow;

pub(crate) async fn run_provider_flow(
    prepared_provider: PreparedProviderFlow,
    transport: &ProviderTransport,
) -> Result<Response<Body>> {
    let provider_http = prepared_provider
        .send_to_upstream(transport)
        .await?
        .handle_upstream_response()
        .await?;

    Ok(provider_http.translate_to_outbound().await?)
}
