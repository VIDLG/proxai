use axum::body::{Body, Bytes};
use axum::http::Response;

use crate::http_support::response_with_headers;
use crate::http_support::{OutboundResponseHead, UpstreamResponseHead};
use crate::observe::{
    ObserveContext, OutboundResponseHeadPrepared, UpstreamNonStreamingResponseReceived,
};

pub(crate) fn forward_non_streaming_response(
    obs: &ObserveContext,
    head: UpstreamResponseHead,
    body: Bytes,
    transform_body: Option<impl FnOnce(Bytes) -> Bytes>,
) -> Response<Body> {
    obs.observe_upstream_non_streaming_success(UpstreamNonStreamingResponseReceived {
        head: &head,
        body: &body,
    });
    let outbound_head = OutboundResponseHead::from_upstream(&head);
    obs.observe_outbound_response_head_prepared(OutboundResponseHeadPrepared {
        head: &outbound_head,
    });

    let (status, headers) = outbound_head.into_parts();
    let body = match transform_body {
        Some(transform_body) => transform_body(body),
        None => body,
    };
    response_with_headers(status, headers, Body::from(body))
}
