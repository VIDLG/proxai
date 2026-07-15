use serde_json::Value;

use crate::error::RequestError;
use crate::protocol::RequestProtocol;
use proxai_core::observe::Observer;

pub(crate) use proxai_core::ingress::PreparedInboundRequest;
use proxai_core::ingress::prepare_inbound_request_with_observer as prepare_structured_request;

pub(crate) fn prepare_inbound_request(
    protocol: RequestProtocol,
    body: &[u8],
    observer: &dyn Observer,
) -> Result<PreparedInboundRequest, RequestError> {
    let payload = serde_json::from_slice::<Value>(body)
        .map_err(|source| RequestError::InvalidJson { protocol, source })?;
    prepare_structured_request(protocol, payload, observer).map_err(Into::into)
}

#[cfg(test)]
mod tests;
