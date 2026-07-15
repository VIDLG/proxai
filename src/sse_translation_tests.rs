use axum::body::Bytes;
use futures_util::{StreamExt, stream};

use crate::http_support::into_byte_stream;

use super::{SseTranslationStreamError, translate_sse_stream};

#[tokio::test]
async fn returns_the_raw_triggering_sse_event_with_the_stream_error() {
    let input = into_byte_stream(stream::iter([Ok::<_, std::io::Error>(Bytes::from_static(
        b"data: {\"type\":\"example\",\"private\":\"redacted\"}\n\n",
    ))]));

    let translator = crate::translation::Translator::new(
        crate::protocol::RequestProtocol::OpenaiResponses,
        crate::protocol::ProviderProtocol::OpenaiChatCompletions,
    );
    let mut translated = translate_sse_stream(input, translator);
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
    let translator = crate::translation::Translator::new(
        crate::protocol::RequestProtocol::AnthropicMessages,
        crate::protocol::ProviderProtocol::OpenaiChatCompletions,
    );

    let mut translated = translate_sse_stream(input, translator);
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
    let translator = crate::translation::Translator::new(
        crate::protocol::RequestProtocol::OpenaiResponses,
        crate::protocol::ProviderProtocol::OpenaiChatCompletions,
    );

    let mut translated = translate_sse_stream(input, translator);
    let error = translated.next().await.unwrap().unwrap_err();

    assert!(matches!(error, SseTranslationStreamError::Upstream(_)));
}
