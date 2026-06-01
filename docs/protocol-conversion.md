# Protocol Conversion and Wire-Model Alignment

ProxAI keeps protocol conversion explicit and pair-oriented. This document records the rules for maintaining wire models, translation code, and SDK-alignment checks.

## Boundaries

- `src/protocol/` owns protocol-specific Rust wire models.
- `src/translation/` owns cross-protocol conversion between an inbound `request_protocol` and an outbound provider `protocol`.
- `src/provider/` owns provider transport and provider-local compatibility behavior.
- `src/ingress/` owns inbound parsing and normalization before translation.

Do not hide general cross-protocol conversion inside a provider subtree. Provider code may normalize provider-local quirks, but protocol-to-protocol shape changes belong in `src/translation/`.

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

A route may specify `request_protocol`. If omitted, it defaults to the selected provider's `protocol`, which means no cross-protocol conversion by default.

Cross-protocol routing should be explicit. This keeps local proxy behavior predictable and avoids accidentally converting requests because a provider default changed.

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
