//! Bedrock Mantle extensions observed on its OpenAI-compatible Responses API.
//!
//! AWS exposes Mantle-only models through `bedrock-mantle`, an inference
//! endpoint separate from Bedrock Runtime's `Converse` / `Invoke` APIs. Its
//! Responses stream is mostly OpenAI-compatible but uses the non-official
//! `response.reasoning.delta/done` channel documented here.
//!
//! Keep these provider-dialect models private to Responses translation ingress:
//! they are not part of the official OpenAI wire union and must never be emitted
//! or advertised as official Responses events. AWS's public Mantle guide does
//! not currently specify this extension's event schema; the compatibility shape
//! comes from Zed `d12b980ee0` (`v1.11.3`), which added native Bedrock Mantle
//! support.

use serde::Deserialize;
use serde_json::Value;
use strum::AsRefStr;

use crate::json::deserialize_value;
use crate::translation::stream::StreamTranslationResult;

const REASONING_DELTA: &str = "response.reasoning.delta";
const REASONING_DONE: &str = "response.reasoning.done";

#[derive(Debug, Deserialize, AsRefStr)]
#[serde(tag = "type")]
pub(crate) enum MantleStreamEvent {
    /// Incremental thinking text. Zed models both official-style output-item
    /// identity fields as optional, and its Mantle test omits both. Downstream
    /// projection may preserve and validate supplied coordinates but must not
    /// invent them.
    #[serde(rename = "response.reasoning.delta")]
    #[strum(serialize = "response.reasoning.delta")]
    ReasoningDelta {
        #[serde(default)]
        item_id: Option<String>,
        #[serde(default)]
        output_index: Option<u32>,
        delta: String,
    },
    /// Terminal snapshot for the same compatibility thinking channel. Zed
    /// models `text` and both output-item identity fields as optional, so target
    /// protocols may need to close an already-open block without reconciling a
    /// final snapshot.
    #[serde(rename = "response.reasoning.done")]
    #[strum(serialize = "response.reasoning.done")]
    ReasoningDone {
        #[serde(default)]
        item_id: Option<String>,
        #[serde(default)]
        output_index: Option<u32>,
        #[serde(default)]
        text: Option<String>,
    },
}

pub(super) fn parse_stream_event(
    payload: &Value,
) -> Option<StreamTranslationResult<MantleStreamEvent>> {
    let context = match payload.get("type").and_then(Value::as_str) {
        Some(REASONING_DELTA) => "Bedrock Mantle reasoning delta event",
        Some(REASONING_DONE) => "Bedrock Mantle reasoning done event",
        _ => return None,
    };
    Some(deserialize_value(payload, context).map_err(Into::into))
}
