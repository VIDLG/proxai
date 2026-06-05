# ProxAI

[中文文档](README_CN.md)

ProxAI is a small local compatibility proxy for OpenAI-compatible requests.
It accepts local client traffic, normalizes the specific OpenAI Responses API
system-message shape that breaks some upstreams, and forwards requests to the
configured provider with minimal surprises.

Today, the stable runtime paths support no-conversion forwarding for OpenAI
Responses, OpenAI Chat Completions, and Anthropic Messages, plus explicit
cross-protocol translation for selected protocol pairs. The config model is
protocol-aware so routing and conversion paths can expand explicitly over time
without turning ProxAI into a generic AI gateway.

## Current Status

The current stable forwarding and translation paths are:

- inbound: `openai_responses` -> outbound: `openai_responses`
- inbound: `openai_chat_completions` -> outbound: `openai_chat_completions`
- inbound: `anthropic_messages` -> outbound: `anthropic_messages`
- inbound: `openai_responses` -> outbound: `openai_chat_completions`
- inbound: `openai_responses` -> outbound: `anthropic_messages`
- inbound: `openai_chat_completions` -> outbound: `anthropic_messages`
- inbound: `anthropic_messages` -> outbound: `openai_responses`

Other cross-protocol translation paths remain intentionally unsupported until
they are implemented explicitly.

## What ProxAI Does Today

For JSON `/v1/responses` requests, ProxAI currently normalizes the specific
system-message shape that breaks some upstreams:

- finds top-level `input` items with `role:"system"`
- extracts `input_text` / `text` parts
- prepends that text to top-level `instructions`
- removes the original system item from `input`
- leaves other fields as unchanged as practical

This keeps clients working with upstreams that reject Responses-style system
messages inside `input`.

For `/v1/chat/completions` requests, ProxAI validates the Chat Completions
request shape, applies provider routing/model rewrite, and either forwards the
request to an OpenAI Chat Completions provider unchanged or translates it to
Anthropic Messages when an explicit route selects an `anthropic_messages`
provider.

For `/v1/messages` requests, ProxAI validates the Anthropic Messages request
shape, applies provider routing/model rewrite, and either forwards the request
to an Anthropic Messages provider unchanged or translates it to OpenAI
Responses when an explicit route selects an `openai_responses` provider.

## Installation and App Directory

Download the Windows release executable, run it once, and then edit the
generated config files in the user app directory.

Generated runtime files live under:

- Windows: `%USERPROFILE%\\.proxai\\config.toml`
- Windows: `%USERPROFILE%\\.proxai\\config.example.toml`
- Linux/macOS: `~/.proxai/config.toml`
- Linux/macOS: `~/.proxai/config.example.toml`

Additional runtime folders under the same app dir include:

- `logs/`
- `captures/`

Before first real use, set the referenced provider `base_url` and `api_key` in
`config.toml`.

## Running

After editing your config:

- executable name: `proxai.exe`
- default proxy listen address: `http://127.0.0.1:18080`
- default MCP endpoint: `http://127.0.0.1:18081/mcp`

CLI overrides remain intentionally small:

- `--config`
- `--upstream`
- `--api-key`
- `--port`
- `--log-level`
- `--log-format`
- `--route-override ROUTE.FIELD=VALUE`

`--upstream` and `--api-key` temporarily override the provider selected by
`routing.default_provider_names.openai_responses` for that run.
`--route-override` temporarily overrides a named `[[routing.routes]]` field for
that run, for example:

```sh
proxai --route-override minimax_m3_chat.model_pattern=MiniMax-M3-preview
```

## Config Overview

Runtime configuration lives in `config.toml`. The tracked reference file is
`config.example.toml`.

For the full field-by-field explanation, see:

- [docs/configuration.md](docs/configuration.md)

In short, the config is organized around:

- `[server]` (listen address plus request body and concurrency limits)
- `[mcp]`
- `[routing.default_provider_names]`
- `[[routing.routes]]`
- `[providers.<name>]`
- `[tool_calls]`
- `[capture]` (`inbound_request_enabled` / `provider_request_enabled` / `upstream_response_enabled` / `outbound_response_enabled`)
- `[logging]`
- `[error_responses]`

Today, the stable runtime paths include no-conversion forwarding for OpenAI
Responses, OpenAI Chat Completions, and Anthropic Messages, plus explicit
translations for `openai_responses -> openai_chat_completions`,
`openai_responses -> anthropic_messages`,
`openai_chat_completions -> anthropic_messages`, and
`anthropic_messages -> openai_responses`. Named routes can be temporarily
adjusted with `--route-override ROUTE.FIELD=VALUE` without editing
`config.toml`.

A route's `request_protocol` is optional. When omitted, the route can match any
inbound endpoint protocol detected from the request path; when set, a matching
model with a different inbound protocol is a configuration error.

For Anthropic Messages providers, use `compatibility = "strict"` with the
official Anthropic API and `compatibility = "anthropic_compatible"` for
compatible upstreams that omit some official response fields.

For upstream non-2xx responses, ProxAI normalizes the response body and preserves
useful diagnostic headers such as `Retry-After`, upstream request ids, and
rate-limit headers.

The `[mcp]` section configures a local MCP listener. By default ProxAI starts a streamable HTTP MCP endpoint at `http://127.0.0.1:18081/mcp`.

## Client Setup Today

For OpenAI-compatible clients, point a provider at:

- `http://127.0.0.1:18080/v1`

Keep model names stable in the client and let ProxAI route them internally.
A practical approach is to expose logical names such as:

- `gpt-5.4`
- `gpt-5.5`
- later, possibly `claude-sonnet`

Do not put real upstream URLs or keys in client settings. Keep upstream details
in `~/.proxai/config.toml`.

## Development

Common commands:

- Rust toolchain: stable with Rust 2024 edition support
- `pixi install`
- `just run`
- `just check`
- `just test-e2e`
- `just build`
- `cargo run -- check-update`

Protocol coverage comparison against official SDKs:

- `just compare-anthropic-protocol` — compare Anthropic Messages protocol types against the official TS SDK
- `just compare-openai-protocol` — compare OpenAI protocol types against `async-openai` v0.40.2

The referenced SDK checkouts are tracked as git submodules under `contrib/`:

- `contrib/anthropic-sdk-typescript`
- `contrib/async-openai`

Use `-d` (detail, default), `-q` (brief), or `-v` (verbose with classification) for output detail.

Useful capture control commands:

- `cargo run -- capture status`
- `cargo run -- capture enable`
- `cargo run -- capture disable`
- `cargo run -- capture enable inbound-request`
- `cargo run -- capture enable provider-request`
- `cargo run -- capture enable upstream-response`
- `cargo run -- capture enable outbound-response`

Useful temporary debug overrides for a single run:

- `cargo run -- --capture-inbound-request`
- `cargo run -- --capture-provider-request`
- `cargo run -- --capture-upstream-response`
- `cargo run -- --capture-outbound-response`

The local release executable is:

- `target\\release\\proxai.exe`

## Protocol Alignment Strategy

ProxAI's protocol types follow a strict **name consistency** rule:

1. **No type aliases** — every SDK type name has exactly one corresponding
   `pub struct` or `pub enum` in proxai, never `pub type X = Y`.
2. **No folded types** — when the SDK distinguishes between `*Block` and
   `*BlockParam` (or similar request/response pairs), proxai maintains
   separate structs for each rather than sharing one with an alias.
3. **No renamed types** — proxai uses the SDK's native name, even when the
   SDK's casing is inconsistent (`Base64PdfSource`, not `Base64PDFSource`).
4. **String unions as enums** — fixed-string unions in the SDK
   (`Array<'direct' | 'code_execution_20250825'>`) are modeled as
   `Vec<EnumType>` rather than `Vec<String>`.

These rules are enforced by `tools/compare_anthropic_protocol.py` and
`tools/compare_openai_protocol.py`, which use tree-sitter AST parsing to
compare proxai types field-by-field against the official SDK. The scripts
report missing types, missing fields, field-order mismatches, serde wire
semantics, and deprecated-field auto-exclusions at three verbosity levels.
When an SDK required-nullable field (`field: T | null`) is represented as a
Rust `Option<T>`, the field must carry
`/// @sdk(required_nullable_accepts_missing)` to document that proxai
intentionally accepts a missing field as compatibility tolerance. See
`docs/protocol-conversion.md` for the full conversion and alignment rules.

## Release Artifacts

GitHub release artifacts are versioned like:

- `proxai-vX.Y.Z-windows-x86_64.exe`

## Notes on Future Protocols

The current repo keeps cross-protocol translation and route-level protocol
filtering explicit. Add new protocol pairs deliberately, with runtime routing,
request/response conversion, and tests for the exact pair, rather than growing
ProxAI into a generic AI platform by accident.
