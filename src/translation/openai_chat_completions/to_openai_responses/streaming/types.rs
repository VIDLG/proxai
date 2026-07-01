//! Internal streaming data structures shared between `state` and `output`.
//!
//! These represent the accumulated content of an in-flight text or tool-call
//! item during a Chat → Responses stream. They are owned and mutated by
//! `state::StreamingState` and consumed by `output` helpers when finalizing
//! terminal events.

#[derive(Debug, Clone)]
pub(super) struct StreamTextItem {
    pub(super) item_id: String,
    pub(super) text: String,
}

impl StreamTextItem {
    pub(super) fn new(item_id: String) -> Self {
        Self {
            item_id,
            text: String::new(),
        }
    }

    pub(super) fn append(&mut self, delta: &str) {
        self.text.push_str(delta);
    }
}

#[derive(Debug, Clone)]
pub(super) struct StreamToolItem {
    pub(super) item_id: String,
    pub(super) name: String,
    pub(super) arguments: String,
}

impl StreamToolItem {
    pub(super) fn new(item_id: String, name: String) -> Self {
        Self {
            item_id,
            name,
            arguments: String::new(),
        }
    }

    pub(super) fn set_name(&mut self, name: &str) {
        self.name = name.to_string();
    }

    pub(super) fn append_arguments(&mut self, delta: &str) {
        self.arguments.push_str(delta);
    }
}
