use std::collections::BTreeMap;

use serde::Serialize;
use strum::Display;

use crate::protocol::anthropic::messages::{
    ContentBlock, ContentBlockDelta, Message, MessageDeltaEvent, MessageStreamEvent, StopReason,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Display, Serialize)]
#[strum(serialize_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub(crate) enum AnthropicResponseOutputKind {
    Text,
    Thinking,
    RedactedThinking,
    ToolUse,
    ToolResult,
    ServerToolUse,
    WebSearchToolResult,
    WebFetchToolResult,
    CodeExecutionToolResult,
    BashCodeExecutionToolResult,
    TextEditorCodeExecutionToolResult,
    ToolSearchToolResult,
    ContainerUpload,
    StreamTextDelta,
    StreamInputJsonDelta,
    StreamCitationsDelta,
    StreamThinkingDelta,
    StreamSignatureDelta,
    StreamContentBlockStart,
    StreamContentBlockStop,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct AnthropicResponseSummary {
    pub(crate) output_items: BTreeMap<AnthropicResponseOutputKind, u64>,
    pub(crate) stop_reasons: BTreeMap<String, u64>,
    pub(crate) tool_uses: BTreeMap<String, u64>,
    pub(crate) server_tool_uses: BTreeMap<String, u64>,
}

impl From<&Message> for AnthropicResponseSummary {
    fn from(message: &Message) -> Self {
        let mut summary = Self::default();
        for block in &message.content {
            summary.record_content_block(block);
        }
        if let Some(reason) = message.stop_reason {
            summary.increment_stop_reason(reason);
        }
        summary
    }
}

impl From<&MessageStreamEvent> for AnthropicResponseSummary {
    fn from(event: &MessageStreamEvent) -> Self {
        let mut summary = Self::default();
        match event {
            MessageStreamEvent::MessageStart(event) => {
                for block in &event.message.content {
                    summary.record_content_block(block);
                }
                if let Some(reason) = event.message.stop_reason {
                    summary.increment_stop_reason(reason);
                }
            }
            MessageStreamEvent::MessageDelta(event) => summary.observe_message_delta(event),
            MessageStreamEvent::ContentBlockStart(event) => {
                summary.increment_item_kind(AnthropicResponseOutputKind::StreamContentBlockStart);
                summary.record_content_block(&event.content_block);
            }
            MessageStreamEvent::ContentBlockDelta(event) => {
                match &event.delta {
                    ContentBlockDelta::TextDelta(_) => {
                        summary.increment_item_kind(AnthropicResponseOutputKind::StreamTextDelta)
                    }
                    ContentBlockDelta::InputJsonDelta(_) => summary
                        .increment_item_kind(AnthropicResponseOutputKind::StreamInputJsonDelta),
                    ContentBlockDelta::CitationsDelta(_) => summary
                        .increment_item_kind(AnthropicResponseOutputKind::StreamCitationsDelta),
                    ContentBlockDelta::ThinkingDelta(_) => summary
                        .increment_item_kind(AnthropicResponseOutputKind::StreamThinkingDelta),
                    ContentBlockDelta::SignatureDelta(_) => summary
                        .increment_item_kind(AnthropicResponseOutputKind::StreamSignatureDelta),
                }
            }
            MessageStreamEvent::ContentBlockStop(_) => {
                summary.increment_item_kind(AnthropicResponseOutputKind::StreamContentBlockStop);
            }
            MessageStreamEvent::Ping(_) | MessageStreamEvent::MessageStop(_) => {}
        }
        summary
    }
}

impl AnthropicResponseSummary {
    pub(crate) fn merge(&mut self, other: &Self) {
        for (kind, count) in &other.output_items {
            *self.output_items.entry(*kind).or_default() += count;
        }
        for (reason, count) in &other.stop_reasons {
            *self.stop_reasons.entry(reason.clone()).or_default() += count;
        }
        for (name, count) in &other.tool_uses {
            *self.tool_uses.entry(name.clone()).or_default() += count;
        }
        for (name, count) in &other.server_tool_uses {
            *self.server_tool_uses.entry(name.clone()).or_default() += count;
        }
    }

    fn observe_message_delta(&mut self, event: &MessageDeltaEvent) {
        if let Some(reason) = event.delta.stop_reason {
            self.increment_stop_reason(reason);
        }
        if let Some(server_tool_use) = event.usage.server_tool_use.as_ref() {
            if server_tool_use.web_search_requests > 0 {
                self.increment_server_tool_use("web_search", server_tool_use.web_search_requests);
            }
            if server_tool_use.web_fetch_requests > 0 {
                self.increment_server_tool_use("web_fetch", server_tool_use.web_fetch_requests);
            }
        }
    }

    fn record_content_block(&mut self, block: &ContentBlock) {
        match block {
            ContentBlock::Text(_) => self.increment_item_kind(AnthropicResponseOutputKind::Text),
            ContentBlock::Thinking(_) => {
                self.increment_item_kind(AnthropicResponseOutputKind::Thinking)
            }
            ContentBlock::RedactedThinking(_) => {
                self.increment_item_kind(AnthropicResponseOutputKind::RedactedThinking)
            }
            ContentBlock::ToolUse(block) => {
                self.increment_item_kind(AnthropicResponseOutputKind::ToolUse);
                *self.tool_uses.entry(block.name.clone()).or_default() += 1;
            }
            ContentBlock::ToolResult(_) => {
                self.increment_item_kind(AnthropicResponseOutputKind::ToolResult)
            }
            ContentBlock::ServerToolUse(block) => {
                self.increment_item_kind(AnthropicResponseOutputKind::ServerToolUse);
                *self
                    .server_tool_uses
                    .entry(block.name.to_string())
                    .or_default() += 1;
            }
            ContentBlock::WebSearchToolResult(_) => {
                self.increment_item_kind(AnthropicResponseOutputKind::WebSearchToolResult)
            }
            ContentBlock::WebFetchToolResult(_) => {
                self.increment_item_kind(AnthropicResponseOutputKind::WebFetchToolResult)
            }
            ContentBlock::CodeExecutionToolResult(_) => {
                self.increment_item_kind(AnthropicResponseOutputKind::CodeExecutionToolResult)
            }
            ContentBlock::BashCodeExecutionToolResult(_) => {
                self.increment_item_kind(AnthropicResponseOutputKind::BashCodeExecutionToolResult)
            }
            ContentBlock::TextEditorCodeExecutionToolResult(_) => self.increment_item_kind(
                AnthropicResponseOutputKind::TextEditorCodeExecutionToolResult,
            ),
            ContentBlock::ToolSearchToolResult(_) => {
                self.increment_item_kind(AnthropicResponseOutputKind::ToolSearchToolResult)
            }
            ContentBlock::ContainerUpload(_) => {
                self.increment_item_kind(AnthropicResponseOutputKind::ContainerUpload)
            }
        }
    }

    fn increment_item_kind(&mut self, kind: AnthropicResponseOutputKind) {
        *self.output_items.entry(kind).or_default() += 1;
    }

    fn increment_stop_reason(&mut self, reason: StopReason) {
        *self.stop_reasons.entry(reason.to_string()).or_default() += 1;
    }

    fn increment_server_tool_use(&mut self, name: &str, count: u32) {
        *self.server_tool_uses.entry(name.to_string()).or_default() += u64::from(count);
    }
}
