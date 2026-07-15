use super::StreamingState;

#[test]
fn rejects_message_delta_before_content_block_stop() {
    let mut state = StreamingState::default();
    state.register_text_block(3).unwrap();

    let error = state.ensure_content_blocks_closed().unwrap_err();
    assert!(
        error.to_string().contains(
            "message_delta before content_block_stop for open text content block index 3"
        )
    );

    assert!(!state.finish_content_block(3).unwrap());
    state.ensure_content_blocks_closed().unwrap();
}

#[test]
fn finishes_ignored_content_block_without_continuation() {
    let mut state = StreamingState::default();
    state.register_ignored_block(2, "server_tool_use").unwrap();

    assert!(!state.finish_content_block(2).unwrap());
    assert_eq!(state.take_continuation(), None);
    state.ensure_content_blocks_closed().unwrap();
}
