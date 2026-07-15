use crate::error::RequestError;
use crate::protocol::RequestProtocol;
use proxai_core::observe::NoopObserver;

use super::prepare_inbound_request;

#[test]
fn rejects_non_json_payload_before_structured_ingress() {
    let error =
        prepare_inbound_request(RequestProtocol::OpenaiResponses, b"not json", &NoopObserver)
            .unwrap_err();

    assert!(matches!(
        error,
        RequestError::InvalidJson {
            protocol: RequestProtocol::OpenaiResponses,
            ..
        }
    ));
}
