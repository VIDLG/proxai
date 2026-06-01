use std::collections::BTreeSet;
use std::pin::Pin;
use std::time::Duration;

use getset::MutGetters;
use serde_json::Value;
use tokio::time::Sleep;

use crate::sse::SseEvent;

use super::sse::{is_tool_argument_delta, is_tool_argument_done};

#[derive(Default, MutGetters)]
pub(super) struct ToolArgumentStreamState {
    pending_items: BTreeSet<String>,
    #[getset(get_mut = "pub(super)")]
    timeout_sleep: Option<Pin<Box<Sleep>>>,
}

impl ToolArgumentStreamState {
    pub(super) fn observe_event(
        &mut self,
        event: &SseEvent,
        timeout: Option<Duration>,
    ) -> Result<(), String> {
        if is_tool_argument_delta(event) {
            self.observe_delta(event, timeout)?;
        }
        if is_tool_argument_done(event) {
            self.observe_done(event, timeout);
        }
        Ok(())
    }

    pub(super) fn clear(&mut self) {
        self.pending_items.clear();
        self.timeout_sleep = None;
    }

    pub(super) fn has_pending_items(&self) -> bool {
        !self.pending_items.is_empty()
    }

    fn observe_delta(&mut self, event: &SseEvent, timeout: Option<Duration>) -> Result<(), String> {
        let item_id = event
            .payload_json()
            .as_ref()
            .and_then(tool_argument_item_id)
            .ok_or_else(|| {
                "upstream Responses SSE tool argument delta missing non-empty item_id".to_string()
            })?;

        self.pending_items.insert(item_id);
        self.reset_timeout(timeout);
        Ok(())
    }

    fn observe_done(&mut self, event: &SseEvent, timeout: Option<Duration>) {
        if let Some(item_id) = event
            .payload_json()
            .as_ref()
            .and_then(tool_argument_item_id)
        {
            self.pending_items.remove(&item_id);
        } else {
            self.pending_items.clear();
        }

        if self.has_pending_items() {
            self.reset_timeout(timeout);
        } else {
            self.timeout_sleep = None;
        }
    }

    fn reset_timeout(&mut self, timeout: Option<Duration>) {
        let Some(timeout) = timeout else {
            self.timeout_sleep = None;
            return;
        };
        if !self.has_pending_items() {
            self.timeout_sleep = None;
            return;
        }
        let deadline = tokio::time::Instant::now() + timeout;
        if let Some(sleep) = self.timeout_sleep.as_mut() {
            sleep.as_mut().reset(deadline);
        } else {
            self.timeout_sleep = Some(Box::pin(tokio::time::sleep_until(deadline)));
        }
    }
}

fn tool_argument_item_id(payload: &Value) -> Option<String> {
    non_empty_string_field(payload, "item_id")
}

fn non_empty_string_field(payload: &Value, key: &str) -> Option<String> {
    payload
        .get(key)
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string)
}

#[cfg(test)]
#[path = "tool_arguments_tests.rs"]
mod tests;
