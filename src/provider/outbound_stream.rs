use std::io;
use std::pin::Pin;

use axum::body::{Body, Bytes};
use axum::http::{HeaderMap, Response, StatusCode};
use futures_util::Stream;

use crate::error::Result;
use crate::upstream::UpstreamResponseHead;

use super::{
    filter_forwardable_headers, BodyObserver, MonitoredBodyStream, UpstreamResponseContext,
};

/// Outcome of building the outbound side of a 2xx upstream response.
///
/// `head`, `outbound_headers`, and `status` are exposed so callers can perform
/// provider-specific wiring (e.g. SSE error-shape normalization, `event: error`
/// encoding) before wrapping the stream in an axum `Response`. The stream has
/// already been wrapped in a [`MonitoredBodyStream`], which drives the
/// [`BodyObserver`] hooks for the lifetime of the response.
pub(crate) struct OutboundStream {
    pub status: StatusCode,
    pub head: UpstreamResponseHead,
    pub outbound_headers: HeaderMap,
    pub stream: Pin<Box<dyn Stream<Item = io::Result<Bytes>> + Send>>,
}

pub(crate) fn outbound_response(
    status: StatusCode,
    headers: HeaderMap,
    body: Body,
) -> Response<Body> {
    let mut response = Response::new(body);
    *response.status_mut() = status;
    *response.headers_mut() = headers;
    response
}

pub(crate) fn streaming_response(
    status: StatusCode,
    headers: HeaderMap,
    stream: impl Stream<Item = io::Result<Bytes>> + Send + 'static,
) -> Response<Body> {
    outbound_response(status, headers, Body::from_stream(stream))
}

/// Build the outbound side of a 2xx upstream response for streaming providers.
///
/// This consolidates the steps shared by every provider's `handle_success_response`:
///
/// 1. Snapshot the upstream response into an [`UpstreamResponseHead`].
/// 2. Capture the raw upstream headers to the configured capture destination.
/// 3. Filter the upstream headers down to the set safe to forward to the client.
/// 4. Capture the outbound headers so the on-disk capture mirrors what the
///    client actually saw.
/// 5. Wrap the reqwest byte stream in a [`MonitoredBodyStream`] driven by the
///    caller-supplied [`BodyObserver`].
///
/// Provider-specific steps stay with the caller: emitting the `Headers` log
/// record (each provider uses a different log type), SSE error-shape
/// normalization, and constructing the final axum [`axum::http::Response`].
pub(crate) async fn build_outbound_stream<O>(
    ctx: &UpstreamResponseContext<'_>,
    upstream_response: reqwest::Response,
    observer: O,
) -> Result<OutboundStream>
where
    O: BodyObserver,
{
    let status = upstream_response.status();
    let upstream_headers = upstream_response.headers().clone();

    let head = UpstreamResponseHead::from_headers(status, &upstream_headers, ctx.started.elapsed());
    ctx.capture
        .capture_upstream_response_headers(&head, &upstream_headers)
        .await?;

    let outbound_headers = filter_forwardable_headers(&upstream_headers);
    ctx.capture
        .capture_outbound_response_headers(
            status,
            head.content_type
                .as_ref()
                .map(ToString::to_string)
                .as_deref(),
            &outbound_headers,
        )
        .await?;

    let capture_writer = ctx
        .capture
        .create_upstream_response_writer(head.content_type.as_ref());
    let stream = MonitoredBodyStream::new(
        upstream_response.bytes_stream(),
        head.clone(),
        ctx.started,
        observer,
        capture_writer,
        ctx.span.clone(),
    );

    Ok(OutboundStream {
        status,
        head,
        outbound_headers,
        stream: Box::pin(stream),
    })
}
