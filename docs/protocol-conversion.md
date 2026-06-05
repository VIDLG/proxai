# Protocol Conversion and Wire-Model Alignment

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
