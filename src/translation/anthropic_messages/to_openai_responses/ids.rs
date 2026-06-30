//! Pair-local stateful allocator for Responses output item ids.
//!
//! Used by both the non-streaming translator (`response.rs`) and the streaming
//! translator (`streaming.rs`) to assign stable, collision-free ids like
//! `msg_<message_id>[_<n>]`, `rs_<message_id>[_<n>]`, `fco_<message_id>[_<n>]`.

#[derive(Debug)]
pub(super) struct OutputItemIdAllocator {
    message_id: String,
    next_message_index: u32,
    next_reasoning_index: u32,
    next_function_call_output_index: u32,
}

impl OutputItemIdAllocator {
    pub(super) fn new(message_id: impl Into<String>) -> Self {
        Self {
            message_id: message_id.into(),
            next_message_index: 0,
            next_reasoning_index: 0,
            next_function_call_output_index: 0,
        }
    }

    pub(super) fn message(&mut self) -> String {
        let id = Self::indexed_id("msg", &self.message_id, self.next_message_index);
        self.next_message_index = self.next_message_index.saturating_add(1);
        id
    }

    pub(super) fn reasoning(&mut self) -> String {
        let id = Self::indexed_id("rs", &self.message_id, self.next_reasoning_index);
        self.next_reasoning_index = self.next_reasoning_index.saturating_add(1);
        id
    }

    pub(super) fn function_call_output(&mut self) -> String {
        let id = Self::indexed_id(
            "fco",
            &self.message_id,
            self.next_function_call_output_index,
        );
        self.next_function_call_output_index =
            self.next_function_call_output_index.saturating_add(1);
        id
    }

    fn indexed_id(prefix: &str, message_id: &str, index: u32) -> String {
        if index == 0 {
            format!("{prefix}_{message_id}")
        } else {
            format!("{prefix}_{message_id}_{index}")
        }
    }
}
