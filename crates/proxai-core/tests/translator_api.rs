use futures_util::{StreamExt, stream};
use proxai_core::protocol::{ProviderProtocol, RequestProtocol};
use proxai_core::translation::Translator;
use proxai_core::translation::stream::{StreamEnd, StreamEvent, StreamTranslationInput};
use serde_json::json;

#[test]
fn translates_values_through_the_public_api() {
    let translator = Translator::new(
        RequestProtocol::OpenaiChatCompletions,
        ProviderProtocol::OpenaiResponses,
    );
    let request = json!({
        "model": "gpt-5.1",
        "messages": [{"role": "user", "content": "hello"}]
    });

    let translated = translator.translate_request(&request).unwrap();

    assert_eq!(translated["model"], "gpt-5.1");
    assert_eq!(translated["input"][0]["role"], "user");
}

#[tokio::test]
async fn translates_structured_streams_through_the_public_api() {
    let translator = Translator::new(
        RequestProtocol::OpenaiChatCompletions,
        ProviderProtocol::OpenaiChatCompletions,
    );
    let input = stream::iter([
        Ok(StreamTranslationInput::Event(StreamEvent::new(
            "message",
            json!({"id": "chatcmpl_1"}),
        ))),
        Ok(StreamTranslationInput::End(StreamEnd::Done)),
    ]);

    let output = translator.translate_stream(input).collect::<Vec<_>>().await;

    assert_eq!(output.len(), 1);
    assert_eq!(output[0].as_ref().unwrap().data["id"], "chatcmpl_1");
}
