# Protocol Conversion and Wire-Model Alignment

中文版本：[`protocol-conversion_cn.md`](protocol-conversion_cn.md)

ProxAI keeps protocol conversion explicit and pair-oriented. This document records the rules for maintaining wire models, translation code, and SDK-alignment checks.

## Boundaries

- `src/protocol/` owns protocol-specific Rust wire models.
- `src/ingress/` owns inbound parsing and normalization before translation.
- `src/translation/` owns pure cross-protocol conversion between an inbound `request_protocol` and an outbound provider `protocol`.
- `src/provider/request.rs` owns provider request preparation, including model rewrite, projection/summary extraction, and JSON body serialization.
- `src/provider/transport.rs` owns outbound HTTP transport, auth headers, upstream URL construction, and send.
- `src/http_support/` owns HTTP carrier helpers such as `ByteStream`, content-type/header helpers, and response reconstruction.

Do not hide general cross-protocol conversion inside a provider subtree. Provider code may normalize provider-local quirks, but protocol-to-protocol shape changes belong in `src/translation/`.

Translation APIs should stay pure at the carrier boundary:

- request translation: `(request_protocol, provider_protocol, normalized_payload) -> payload`
- non-streaming response translation: `(request_protocol, provider_protocol, payload) -> payload`
- streaming response translation: `(request_protocol, provider_protocol, ByteStream) -> ByteStream`

Do not pass HTTP `Response`, `Body`, provider request structs, or route/model rewrite details into `src/translation/`.

## Naming

Use protocol names for wire behavior:

- `openai_responses`
- `openai_chat_completions`
- `anthropic_messages`

Use pair-oriented conversion module names, for example:

- `openai_responses -> anthropic_messages`
- `anthropic_messages -> openai_responses`

Provider names are user labels and should not be treated as semantic protocol identifiers.

## Routing and conversion

A route may specify `request_protocol`. If omitted, the route can match any
inbound request protocol detected from the actual request path. Provider
`protocol` controls the outbound wire format, so route protocol filtering and
protocol conversion are separate decisions.

Set `request_protocol` only when the same model pattern needs different routing
for different request endpoints. If a model pattern matches but the explicit
`request_protocol` differs from the inbound request protocol, ProxAI reports a
configuration error instead of silently falling through to a default provider.

## OpenAI Chat ↔ Anthropic Messages message placement

OpenAI Chat Completions and Anthropic Messages both model a conversation as
ordered turns, but they place system instructions, tool calls, and tool results
in different parts of the request body. Keep these placement rules explicit in
translation code.

### High-level placement

| Concept | OpenAI Chat Completions | Anthropic Messages |
| --- | --- | --- |
| System instructions | `messages[]` item with `role: "system"` | top-level `system` field |
| Developer instructions | `messages[]` item with `role: "developer"` | no dedicated role; fold into top-level `system` |
| User content | `messages[]` item with `role: "user"` | `messages[]` item with `role: "user"` |
| Assistant text | `messages[]` item with `role: "assistant"` and `content` | `messages[]` item with `role: "assistant"` and text content blocks |
| Tool call request | assistant message `tool_calls[]` | assistant message content block with `type: "tool_use"` |
| Tool call result | separate `messages[]` item with `role: "tool"` and `tool_call_id` | user message content block with `type: "tool_result"` |
| Legacy function result | separate `messages[]` item with `role: "function"` | unsupported; no reliable `tool_result` mapping without `tool_call_id` |

### System and developer instructions

Chat keeps system-like instructions inside the ordered `messages[]` array:

```json
{"role": "system", "content": "You are concise."}
{"role": "developer", "content": "Prefer exact answers."}
```

Anthropic has no `developer` role and does not put system instructions in
`messages[]`. Translate Chat `system` and `developer` content into the top-level
Anthropic `system` field. If there is a single non-empty text part, use the
string form. If there are multiple parts, use the block form to preserve
boundaries:

```json
{
  "system": [
    {"type": "text", "text": "You are concise."},
    {"type": "text", "text": "Prefer exact answers."}
  ]
}
```

### User content

Chat `role: "user"` content does not contain tool results. It contains ordinary
user-provided content parts such as text, images, audio, or files:

```json
{
  "role": "user",
  "content": [
    {"type": "text", "text": "Summarize this."},
    {"type": "image_url", "image_url": {"url": "https://example.test/a.png"}}
  ]
}
```

Translate these into Anthropic user `content` text/image/document blocks where
the target protocol can represent the source. Unsupported user parts should fail
with a `TranslationError::InvalidPayload` rather than being silently dropped.

### Tool call request

In Chat Completions, a model requests tool execution from an assistant message
via `tool_calls[]`:

```json
{
  "role": "assistant",
  "content": "I will look that up.",
  "tool_calls": [
    {
      "id": "call_1",
      "type": "function",
      "function": {
        "name": "lookup",
        "arguments": "{\"query\":\"proxai\"}"
      }
    }
  ]
}
```

In Anthropic Messages, the same request is an assistant content block:

```json
{
  "role": "assistant",
  "content": [
    {"type": "text", "text": "I will look that up."},
    {
      "type": "tool_use",
      "id": "call_1",
      "name": "lookup",
      "input": {"query": "proxai"}
    }
  ]
}
```

Chat function tool arguments are JSON encoded as a string. When translating to
Anthropic `tool_use.input`, parse that string as JSON and fail the conversion if
it is invalid. Do not replace invalid arguments with `{}`.

Chat function tools map to Anthropic custom tools because both carry a named
JSON-schema input contract. Chat custom tools are different: their input is
freeform text or grammar-constrained text, not a JSON object described by
`input_schema`. Reject Chat custom tool definitions, custom tool choices, and
custom tool calls when translating to Anthropic Messages rather than pretending
that they are empty-object JSON tools.

### Tool call result

In Chat Completions, tool execution output is not part of the assistant message.
It is a separate message with `role: "tool"`:

```json
{
  "role": "tool",
  "tool_call_id": "call_1",
  "content": "found"
}
```

In Anthropic Messages, tool results are user-side content blocks that reference
the earlier `tool_use.id`:

```json
{
  "role": "user",
  "content": [
    {
      "type": "tool_result",
      "tool_use_id": "call_1",
      "content": "found",
      "is_error": false
    }
  ]
}
```

This means a Chat `role: "tool"` message translates to an Anthropic
`role: "user"` message containing a `tool_result` block. Do not try to place
Chat tool results inside Chat user content or Anthropic assistant content.

### Legacy function messages

Chat has legacy function-calling shapes in addition to modern `tool_calls`.
Reject `role: "function"` messages when translating to Anthropic Messages.
Legacy function result messages carry a function name but no stable
`tool_call_id`, while Anthropic `tool_result` blocks must reference the earlier
`tool_use.id`. Do not invent an id or downgrade the result into ordinary user
text.

### Response choices and candidate replies

Chat Completions response `choices[]` is a list of alternative candidate
assistant replies, commonly produced by request parameters such as `n`. It is
not a list of content blocks and it is not the representation for parallel tool
calls.

```json
{
  "choices": [
    {"index": 0, "message": {"role": "assistant", "content": "Option A"}},
    {"index": 1, "message": {"role": "assistant", "content": "Option B"}}
  ]
}
```

Parallel tool calls live inside one candidate assistant message as
`choices[i].message.tool_calls[]`; those can map to multiple Anthropic
`tool_use` blocks in a single assistant message.

Anthropic Messages has no equivalent top-level candidate-list response shape. A
non-streaming Anthropic response is one `Message` with one `content[]` sequence,
not a list of alternative assistant messages. OpenAI Responses API also has no
Chat-style `choices[]` equivalent: its `output[]` is a sequence of output items
(message, function call, reasoning item, and so on), not a set of candidate
answers.

Do not merge multiple Chat choices into one Anthropic `content[]` array and do
not silently keep only the first choice. Both approaches lose protocol
semantics: per-choice `index`, independent `finish_reason`, and the fact that
the choices are alternatives rather than one assistant turn. When translating a
Chat response to Anthropic Messages, require exactly one choice and reject
multi-choice responses.

### Chat -> Anthropic response and stream semantics

For non-streaming Chat -> Anthropic response conversion:

- map `choices[0].message.content` to Anthropic `text` blocks;
- map function `tool_calls[]` to Anthropic `tool_use` blocks, parsing Chat
  function `arguments` as JSON for `tool_use.input`;
- when `message.refusal` is present, keep the visible refusal wording as a
  `text` block and also set `stop_reason: "refusal"` with
  `stop_details.explanation`; Chat has no refusal category, so leave it absent;
- require exactly one Chat choice and reject responses without representable
  text, refusal, or function tool calls.

For streaming Chat -> Anthropic conversion, keep an explicit lifecycle:

1. wait for the first assistant choice chunk before emitting Anthropic
   `message_start`;
2. translate Chat `delta.content` / `delta.refusal` into an Anthropic text block;
   the first text fragment may be carried by `content_block_start`, while later
   fragments use `text_delta`;
3. translate Chat function tool-call starts to `tool_use` block starts with an
   empty object `input`, because Chat streaming `function.arguments` are partial
   JSON strings; send those argument fragments as `input_json_delta` events;
4. when Chat `finish_reason` arrives, close all open content blocks and retain a
   pending terminal state containing the finish reason and refusal wording;
5. emit Anthropic `message_delta` / `message_stop` when a later `choices: []`
   usage-only chunk arrives, or when `[DONE]` / EOF ends the stream without final
   usage.

OpenAI's final streaming usage, when requested with
`stream_options: {"include_usage": true}`, is represented by a final
`choices: []` chunk. Treat that usage-only chunk as the source of final usage.
Do not treat `usage` on a non-empty `choices` chunk as final usage and do not use
it to stop the Anthropic stream. Some OpenAI-compatible servers expose
continuous/intermediate usage statistics on ordinary chunks; those values are
not a replacement for the final usage-only chunk and are ignored by this
conversion.

A `choices: []` Chat stream chunk is only valid as a usage-only chunk after a
terminal `finish_reason` has been seen. Reject usage-only chunks before any
assistant message, before a terminal finish reason, or after the Anthropic
message has stopped. Reject Chat stream `logprobs`, non-assistant delta roles,
and multi-choice chunks rather than silently dropping information Anthropic
Messages cannot represent.

## Refusal and normal content semantics

`refusal` means model-generated refusal content, not an additional annotation on ordinary assistant text. Keep it separate from normal text when translating between protocols.

The three supported protocols represent this separation differently:

| Protocol | Normal assistant text | Refusal | Can normal text and refusal coexist in one assistant message? |
| --- | --- | --- | --- |
| `openai_responses` | `output[].content[]` part with `type: "output_text"` | `output[].content[]` part with `type: "refusal"` | Structurally possible as different content parts, but semantically unusual; preserve order when the target can express parts. |
| `openai_chat_completions` | `choices[].message.content` or stream `delta.content` | `choices[].message.refusal` or stream `delta.refusal` | Wire fields are both nullable/optional, but a refusal should not duplicate the same text in `content`. Assistant request content parts also document either one or more `text` parts, or exactly one `refusal` part. |
| `anthropic_messages` | `content[]` `text` blocks | `stop_reason: "refusal"` plus optional `stop_details.explanation`; visible refusal wording may also arrive as `text` blocks | There is no separate refusal content block. A refused message can still contain visible text blocks, so translators must decide whether those text blocks are refusal text or ordinary content from context. |

### OpenAI Responses

Responses keeps message content as typed parts, so normal text and refusal are separate values in the same `content[]` array:

```json
{
  "type": "message",
  "role": "assistant",
  "status": "completed",
  "content": [
    {
      "type": "output_text",
      "text": "I can help with safe alternatives.",
      "annotations": []
    },
    {
      "type": "refusal",
      "refusal": "I can't provide instructions for that request."
    }
  ]
}
```

If a target protocol can preserve typed content parts, keep the distinction. If the target is Chat Completions, avoid merging the refusal text into ordinary `message.content` unless there is no target-side refusal field available.

### OpenAI Chat Completions

Chat response messages expose ordinary content and refusal as sibling fields:

```json
{
  "role": "assistant",
  "content": null,
  "refusal": "I can't provide instructions for that request."
}
```

The JSON shape does not make `content` and `refusal` mutually exclusive at the top level, but their meanings are different. Do not emit the same refusal text in both fields:

```json
{
  "role": "assistant",
  "content": "I can't provide instructions for that request.",
  "refusal": "I can't provide instructions for that request."
}
```

Treat that duplicated shape as a compatibility artifact to avoid producing, not as desired output.

Assistant request content parts make the separation more explicit: an array can contain one or more `text` parts, or exactly one `refusal` part. That reinforces the semantic rule that refusal is an alternative content kind, not a decoration on normal text.

Streaming has the same split:

```text
data: {"choices":[{"index":0,"delta":{"refusal":"I can't help with that."},"finish_reason":null}]}
data: {"choices":[{"index":0,"delta":{},"finish_reason":"stop"}]}
data: [DONE]
```

If ordinary `delta.content` has already been forwarded and a later upstream event reveals the turn was a refusal, the stream cannot be retracted. In that case, do not send duplicate refusal text; only use `delta.refusal` when the refusal can be represented before ordinary content has been emitted.

### Anthropic Messages

Anthropic does not have a dedicated refusal content block. The visible refusal
wording is still ordinary `content[]` text; the refusal semantics are carried by
message-level stop fields:

- visible wording: `content[]` `text` block;
- refusal marker: `stop_reason: "refusal"`;
- optional refusal metadata: `stop_details`, such as `explanation` and provider
  category.

That differs from Chat Completions, where refusal wording has a sibling field
next to normal content: `choices[].message.refusal`. In Chat, `message.content`
and `message.refusal` are separate content slots. In Anthropic, both ordinary
assistant text and visible refusal wording use the same `text` block shape, and
only the message-level stop fields tell the translator whether those text blocks
should become Chat `message.content` or Chat `message.refusal`.

A refusal is identified at message level:

```json
{
  "id": "msg_01",
  "type": "message",
  "role": "assistant",
  "content": [
    {
      "type": "text",
      "text": "I can't provide instructions for that request."
    }
  ],
  "stop_reason": "refusal",
  "stop_details": {
    "category": "safety",
    "explanation": "The request asks for unsafe instructions."
  }
}
```

For Anthropic -> Chat Completions non-streaming conversion:

- when `stop_reason == "refusal"` and visible text blocks exist, put the flattened visible text in `message.refusal` and leave `message.content` absent/null;
- when `stop_reason == "refusal"` and no visible text exists, use `stop_details.explanation` as the fallback `message.refusal`;
- do not map `stop_details.category` because Chat Completions has no equivalent field;
- map the choice `finish_reason` to `stop`, because a refusal is a terminal assistant turn, not a tool call.

Example target Chat response:

```json
{
  "choices": [
    {
      "index": 0,
      "message": {
        "role": "assistant",
        "refusal": "I can't provide instructions for that request."
      },
      "finish_reason": "stop"
    }
  ]
}
```

For Anthropic -> Chat Completions streaming conversion, `message_delta.stop_reason` and `stop_details` arrive after content block deltas. Therefore proxai uses a best-effort rule:

- map Anthropic `thinking` block text and `thinking_delta` fragments to the Zed-supported Chat-compatible extension field `delta.reasoning_content`; do not put thinking text in ordinary `delta.content`;
- ignore `signature_delta` and `redacted_thinking` payloads instead of leaking them into Chat content, because Chat Completions has no standard safe field for those values;
- if no text delta has been emitted, convert `stop_details.explanation` to `delta.refusal`;
- if text has already been emitted as `delta.content`, do not emit duplicate refusal text;
- still map the final choice `finish_reason` to `stop`.

This is less strict than buffering the entire stream, but it preserves low-latency streaming and avoids retracting already-forwarded content.

## SDK alignment

The Anthropic Messages wire model is compared against the vendored official TypeScript SDK under `contrib/anthropic-sdk-typescript` using:

```sh
just compare-anthropic-protocol
```

The comparison checks type coverage, field coverage and order, serde discriminator handling, enum literals, untagged unions, structured SDK markers, and selected serde field semantics.

## Required-nullable fields

TypeScript distinguishes these two shapes:

```ts
field?: T          // optional: the field may be absent
field: T | null    // required nullable: the field should be present, but may be null
```

Rust `Option<T>` accepts both missing and `null` during deserialization, so it is stricter than neither shape. It is exact enough for SDK-optional fields, but it is intentionally wider than SDK required-nullable fields.

When an SDK required-nullable field is represented as `Option<T>`, mark the Rust field directly:

```rust
pub struct Usage {
    pub output_tokens: u32,
    /// @sdk(required_nullable_accepts_missing)
    pub server_tool_use: Option<ServerToolUsage>,
}
```

This marker means:

- SDK shape: `field: T | null`
- Rust shape: `Option<T>`
- Intentional difference: ProxAI also accepts a missing field as compatibility tolerance

Do not use this marker when the SDK field is optional (`field?: T` or `field?: T | null`). Missing is already part of the official shape there.

Do not use this marker to justify `Option<T>` for SDK required non-null fields (`field: T`). Those should remain non-optional in Rust unless there is a separate, explicitly documented protocol decision.

The compare script prints marked fields compactly in the `Required-nullable fields accepting missing` section. Unmarked required-nullable `Option<T>` fields fail the comparison.

## Compatibility normalization

Provider compatibility normalization should repair only conservative or measured upstream deviations into the nearest official protocol shape. Current conservative repairs are SDK required-nullable response fields missing from JSON objects (`missing -> null`) and bare `message_start` events normalized into the official nested `message` shape. Current measured provider repairs:

- MiniMax-compatible streams may omit `signature` on a thinking `content_block_start`, so ProxAI inserts an empty signature for that narrow case.
- GLM 5.1 Anthropic-compatible streams may emit `server_tool_use` with only one counter, so ProxAI fills the absent `web_fetch_requests` or `web_search_requests` counter with `0`.

Do not add other provider-specific business defaults, such as missing tool callers, unless a measured upstream case and a focused fixture document the behavior.

Keep these repairs local to provider compatibility handling. They should not redefine the official wire model.

## Documentation expectations

When protocol conversion or wire-model alignment rules change:

1. Update this document.
2. Update the relevant protocol document under `docs/protocol-*.md` if behavior changes for users or examples.
3. Update `README.md` and `README_CN.md` when the change affects user-facing development workflow or configuration.
