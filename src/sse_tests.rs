use super::{sse_event_stream, sse_frame_stream, SseEvent, SseEventScanner, SseSegment};
use bytes::Bytes;
use futures_util::{stream, StreamExt};

#[tokio::test]
async fn frame_stream_yields_completed_frames_and_flushes_tail() {
    let segments = sse_frame_stream(stream::iter([
        Ok::<_, std::io::Error>(Bytes::from_static(b"data: a\n")),
        Ok(Bytes::from_static(b"\ndata: b\n\ntail")),
    ]))
    .collect::<Vec<_>>()
    .await
    .into_iter()
    .collect::<Result<Vec<_>, _>>()
    .unwrap();

    assert_eq!(segments.len(), 3);
    let SseSegment::Frame(first) = &segments[0] else {
        panic!("expected first complete frame");
    };
    let SseSegment::Frame(second) = &segments[1] else {
        panic!("expected second complete frame");
    };
    let SseSegment::Tail(tail) = &segments[2] else {
        panic!("expected EOF tail");
    };
    assert_eq!(first.bytes().as_ref(), b"data: a\n\n");
    assert_eq!(second.bytes().as_ref(), b"data: b\n\n");
    assert_eq!(tail.as_ref(), b"tail");
}

#[tokio::test]
async fn event_stream_decodes_complete_frames_and_ignores_tail() {
    let events = sse_event_stream(stream::iter([
        Ok::<_, std::io::Error>(Bytes::from_static(b"event: custom\n")),
        Ok(Bytes::from_static(b"data: one\n\ntail")),
    ]))
    .collect::<Vec<_>>()
    .await
    .into_iter()
    .collect::<Result<Vec<_>, _>>()
    .unwrap();

    assert_eq!(
        events,
        vec![SseEvent {
            event_type: "custom".to_string(),
            data: "one".to_string(),
        }]
    );
}

#[test]
fn frame_to_event_decodes_comments_retry_and_multiline_data() {
    let frame = super::SseFrame::new(Bytes::from_static(
        b": keepalive\nretry: 1000\nevent: custom\ndata: one\ndata: two\n\n",
    ));

    let event = Option::<SseEvent>::try_from(&frame).unwrap().unwrap();

    assert_eq!(event.event_type, "custom");
    assert_eq!(event.data, "one\ntwo");
}

#[test]
fn payload_json_parses_json_event_bodies() {
    let event = SseEvent {
        event_type: "response.output_item.done".to_string(),
        data: "{\"type\":\"response.output_item.done\",\"id\":\"item_1\"}".to_string(),
    };

    let payload = event.payload_json().unwrap();

    assert_eq!(payload["type"], "response.output_item.done");
    assert_eq!(payload["id"], "item_1");
}

#[test]
fn matches_type_or_data_checks_explicit_event_name_and_payload_type() {
    let explicit = SseEvent {
        event_type: "message_stop".to_string(),
        data: "{}".to_string(),
    };
    let data_only = SseEvent {
        event_type: SseEvent::DEFAULT_EVENT_TYPE.to_string(),
        data: "{\"type\":\"response.completed\"}".to_string(),
    };
    let mentions_type_in_non_type_field = SseEvent {
        event_type: SseEvent::DEFAULT_EVENT_TYPE.to_string(),
        data: "{\"delta\":\"response.completed\"}".to_string(),
    };

    assert!(explicit.matches_type_or_data("message_stop"));
    assert!(data_only.matches_type_or_data("response.completed"));
    assert!(!mentions_type_in_non_type_field.matches_type_or_data("response.completed"));
}

#[test]
fn ignores_retry_commands_and_keeps_done_sentinels() {
    let mut scanner = SseEventScanner::default();

    let events = scanner.scan(b"retry: 1500\n\n\ndata: [DONE]\n\n");

    assert_eq!(
        events,
        vec![SseEvent {
            event_type: SseEvent::DEFAULT_EVENT_TYPE.to_string(),
            data: "[DONE]".to_string(),
        }]
    );
    assert!(events[0].is_done_sentinel());
}

#[test]
fn keeps_default_message_events_and_explicit_event_names() {
    let mut scanner = SseEventScanner::default();

    let events = scanner.scan(
        b"data: plain\n\n\
          event: response.completed\n\
          data: {\"type\":\"response.completed\"}\n\n",
    );

    assert_eq!(
        events,
        vec![
            SseEvent {
                event_type: SseEvent::DEFAULT_EVENT_TYPE.to_string(),
                data: "plain".to_string(),
            },
            SseEvent {
                event_type: "response.completed".to_string(),
                data: "{\"type\":\"response.completed\"}".to_string(),
            },
        ]
    );
}

#[test]
fn handles_fragmented_chunks_across_event_boundaries() {
    let mut scanner = SseEventScanner::default();

    let first = scanner.scan(b"event: response.output_text.delta\n");
    let second = scanner.scan(b"data: {\"delta\":\"hel");
    let third = scanner.scan(b"lo\"}\n\n");

    assert!(first.is_empty());
    assert!(second.is_empty());
    assert_eq!(
        third,
        vec![SseEvent {
            event_type: "response.output_text.delta".to_string(),
            data: "{\"delta\":\"hello\"}".to_string(),
        }]
    );
}
