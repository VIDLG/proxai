use axum::body::{Body, Bytes, to_bytes};
use futures_util::{StreamExt, stream};
use proxai_core::pipeline::ResponsePipeline;
use proxai_core::provider::{ProviderBehavior, ProviderCompatibility};

use crate::http_support::into_byte_stream;
use crate::protocol::{ProviderProtocol, RequestProtocol};

use super::{SseTranslationStreamError, translate_sse_stream};

fn response_pipeline(
    request_protocol: RequestProtocol,
    provider_protocol: ProviderProtocol,
) -> ResponsePipeline {
    ResponsePipeline::new(
        request_protocol,
        ProviderBehavior::new(provider_protocol, ProviderCompatibility::Compatible),
    )
}

#[tokio::test]
async fn normalizes_compatible_identity_stream_before_forwarding() {
    let input = into_byte_stream(stream::iter([Ok::<_, std::io::Error>(Bytes::from_static(
        b"data: {\"id\":\"chatcmpl_minimax\",\"object\":\"chat.completion.chunk\",\"created\":1,\"model\":\"MiniMax-M3\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\"hello\"}}]}\n\ndata: [DONE]\n\n",
    ))]));

    let body = to_bytes(
        Body::from_stream(translate_sse_stream(
            input,
            response_pipeline(
                RequestProtocol::OpenaiChatCompletions,
                ProviderProtocol::OpenaiChatCompletions,
            ),
        )),
        usize::MAX,
    )
    .await
    .unwrap();
    let text = String::from_utf8(body.to_vec()).unwrap();

    assert!(text.contains("\"finish_reason\":null"));
    assert!(text.ends_with("data: [DONE]\n\n"));
    assert!(!text.contains("event: message"));
}

#[tokio::test]
async fn returns_the_raw_triggering_sse_event_with_the_stream_error() {
    let input = into_byte_stream(stream::iter([Ok::<_, std::io::Error>(Bytes::from_static(
        b"data: {\"type\":\"example\",\"private\":\"redacted\"}\n\n",
    ))]));

    let mut translated = translate_sse_stream(
        input,
        response_pipeline(
            RequestProtocol::OpenaiResponses,
            ProviderProtocol::OpenaiChatCompletions,
        ),
    );
    let error = translated.next().await.unwrap().unwrap_err();
    let SseTranslationStreamError::Translation(failure) = error else {
        panic!("expected a structured translation failure");
    };

    assert!(failure.to_string().contains("stream translation error"));
    assert_eq!(failure.stage.as_ref(), "event");
    let event = failure.upstream_event.as_ref().unwrap();
    assert_eq!(event.event_type, "message");
    assert_eq!(
        event.data,
        "{\"type\":\"example\",\"private\":\"redacted\"}"
    );
}

#[tokio::test]
async fn finish_error_retains_the_last_upstream_event_and_end_kind() {
    let chunk = "{\"id\":\"chatcmpl_unfinished\",\"object\":\"chat.completion.chunk\",\"created\":1,\"model\":\"gpt-test\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\"partial\"},\"finish_reason\":null}]}";
    let input = into_byte_stream(stream::iter([Ok::<_, std::io::Error>(Bytes::from(
        format!("data: {chunk}\n\ndata: [DONE]\n\n"),
    ))]));

    let mut translated = translate_sse_stream(
        input,
        response_pipeline(
            RequestProtocol::AnthropicMessages,
            ProviderProtocol::OpenaiChatCompletions,
        ),
    );
    let error = loop {
        let item = translated
            .next()
            .await
            .expect("stream must report finish failure");
        if let Err(error) = item {
            break error;
        }
    };
    let SseTranslationStreamError::Translation(failure) = error else {
        panic!("expected a structured translation failure");
    };

    assert_eq!(failure.stage.as_ref(), "finish");
    assert_eq!(
        failure.end,
        Some(crate::translation::stream::StreamEnd::Done)
    );
    assert_eq!(failure.upstream_event.unwrap().data, chunk);
}

#[tokio::test]
async fn preserves_upstream_stream_errors_instead_of_reporting_eof() {
    let input = into_byte_stream(stream::iter([Err::<Bytes, _>(std::io::Error::other(
        "upstream body failed",
    ))]));

    let mut translated = translate_sse_stream(
        input,
        response_pipeline(
            RequestProtocol::OpenaiResponses,
            ProviderProtocol::OpenaiChatCompletions,
        ),
    );
    let error = translated.next().await.unwrap().unwrap_err();

    assert!(matches!(error, SseTranslationStreamError::Upstream(_)));
}
