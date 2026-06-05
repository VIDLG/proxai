# Agent Guidelines Reference

This file preserves lower-frequency guidance for AI agents working in `proxai`.
Keep `AGENTS.md` short because it is loaded into every agent session.

## Full Configuration Context

Runtime configuration is centered on `config.toml`. The tracked default/example structure lives in `config.example.toml`; on first run, the shim generates both files in the user's home `.proxai` directory.

Current config concepts include:

- local server host/port
- routing defaults by inbound request protocol
- route filters by request protocol / model / match kind
- providers with explicit upstream protocol, base URL, API key, and read-idle timeout
- logging level / format / thresholds
- upstream error response format
- capture switches for inbound requests, provider requests, upstream responses, and outbound responses, with captures written to the app-dir `captures/`
- semantic tool-call timeout

CLI options should stay limited to common temporary overrides: `--config`, `--upstream`, `--api-key`, `--port`, `--log-level`, and `--log-format`.
Do not expose every config field as a CLI flag unless there is a clear operational need.

## Provider / Protocol Details

Prefer protocol-based naming where wire behavior differs.

- Use `request_protocol` for inbound route filtering.
- Use provider `protocol` for outbound translation choice.
- Keep provider names as user-defined labels, not semantic protocol identifiers.

Current protocol values include:

- `openai_responses`
- `openai_chat_completions`
- `anthropic_messages`

If a route omits `request_protocol`, it can match any inbound protocol detected from the actual request path. Provider `protocol` still controls outbound wire behavior. Set `request_protocol` only when the same model pattern needs different routes per request endpoint; a model match with a mismatched explicit `request_protocol` should be treated as a configuration error.

## Chain Phase Naming Details

The proxy has a phase axis separate from protocol names:

- `inbound_request`
- `provider_request`
- `upstream_response`
- `outbound_response`

Use phase names for where data sits in the proxy pipeline, for example capture config/artifacts, CLI and MCP control surfaces, logging fields, and proxy-flow locals/helpers.
Do not mix protocol names and phase names into one overloaded concept when a two-axis model is clearer.

Preferred mental model:

- `inbound_request.protocol` = what the client sent
- `provider_request.protocol` = what proxai sent upstream
- `upstream_response.protocol` = what the provider returned
- `outbound_response.protocol` = what proxai returned to the client

Routing and translation connect these axes:

- phase axis: inbound -> forwarded -> upstream -> outbound
- protocol axis: responses / chat_completions / messages

## Translation Layer Details

Keep explicit cross-protocol conversion logic in `src/translation/`.

- `ingress/` owns inbound protocol parsing and normalization.
- `translation/` owns protocol-to-protocol conversion between `request_protocol` and provider `protocol`.
- `provider/request` owns provider request preparation: provider model rewrite, projection/summary extraction, and body serialization.
- `provider/transport` owns outbound HTTP transport: auth headers, upstream URL construction, and send.
- `http_support` owns HTTP carrier helpers: boxed byte streams, content-type helpers, response head/body reconstruction, and forwardable header filtering.

Do not bury general protocol translation inside a provider subtree unless it is truly private to that provider implementation.
Prefer pair-oriented naming such as `openai_responses -> anthropic_messages` over provider-label-oriented naming.

Translation functions should stay pure at the carrier boundary:

- request translation: `(request_protocol, provider_protocol, normalized_payload) -> payload`
- non-streaming response translation: `(request_protocol, provider_protocol, payload) -> payload`
- streaming response translation: `(request_protocol, provider_protocol, ByteStream) -> ByteStream`

Do not pass HTTP `Response`, `Body`, route/model rewrite details, or provider request structs into `translation/`.

Do not force phase naming into unrelated protocol or error domain types when it harms clarity. Names such as `RequestProtocol`, `UpstreamError`, and `UpstreamResponseHead` can remain protocol/domain-oriented when not expressing chain position.

Within tightly scoped provider submodules, concise local filenames like `request.rs` and `response.rs` are fine as long as exported names or surrounding code keep phase semantics clear.

For protocol-specific request/response data, prefer a top-level enum keyed by protocol that wraps concrete per-protocol structs. Avoid structs containing parallel `protocol` / `payload` / `projection` / `summary` fields that can drift into impossible states.

## Logging Details

Logs should stay compact and useful for real client/proxy debugging.

- Prefer structured events and the custom compact formatter.
- Do not inject raw ANSI escape strings into log messages.
- Keep wording stable and short.

Current event tokens intentionally include:

- `fwd`
- `hdr`
- `end`
- `closed`
- `timeout`
- `unfinished-tool`
- `stream-error`

Token usage logs should remain explicit:

- `i` = input tokens
- `o` = output tokens
- `t` = total tokens
- `c` = cached input tokens
- `f` = fresh / uncached input tokens

Do not log request bodies, Authorization headers, API keys, private prompts, or unnecessary private upstream URL details.

## Error Handling Details

For readability, `error_responses.format = "text"` should remain the default.

Use domain-specific errors rather than broad catch-all conversions:

- `RequestError` for inbound body/validation failures.
- `ConfigError` for config loading and config-file reads.
- `InternalError` for proxy runtime invariants, local filesystem IO, internal HTTP body reads, JSON serialization, and translation boundary errors.
- `UpstreamError` for upstream send/status/body-read failures.
- `TranslationError` for protocol payload conversion.
- `SseError` / `SseTranslationError` for SSE event semantics.
- `ByteStreamError` for stream carrier errors.

Avoid wrapping semantic stream or HTTP errors in `std::io::Error`; reserve `io::Error` for real OS/filesystem IO.

When normalizing upstream errors:

- text mode should be concise and human-readable
- JSON mode should remain OpenAI-style for non-Zed clients
- preserve useful headers such as `Retry-After`

Do not overfit parsing to every possible upstream JSON shape. Prefer behavior-level tests proving that clients receive useful errors.

## SSE / Streaming Details

Streaming behavior is user-visible and easy to regress. Be careful with:

- keeping streams as `ByteStream` once they cross the HTTP carrier boundary
- preserving SSE response body bytes
- preserving `text/event-stream`
- detecting terminal events
- stalled tool-call argument streams
- Unicode chunk scanning without slicing panics

Keep the semantic timeout configurable via `[tool_calls].timeout_secs`.

## Test Philosophy Details

Favor high-value tests over exhaustive low-value snapshots.

Good tests:

- cover real proxy behavior through `tests/proxy_e2e.rs`
- protect client-facing regressions
- cover generated app-directory defaults and config loading behavior
- cover system-message normalization behavior
- cover SSE stalls, incomplete tool streams, and Unicode stream scanning
- cover route matching and protocol-aware config behavior where practical

Avoid or remove low-value tests that only:

- snapshot log formatting strings expected to evolve
- test private helpers without user-visible behavior
- duplicate existing E2E coverage
- lock incidental implementation details

Adjacent test files are preferred for module tests when that keeps ownership clear. For Rust module tests, keep `*_tests.rs` adjacent to the implementation file and include it from the owning module, not from the parent `mod.rs`. Put the test-module declaration at the bottom of the owning file so core code stays prominent:

```rust
#[cfg(test)]
#[path = "foo_tests.rs"]
mod tests;
```

For example, `foo.rs` owns `foo_tests.rs`; parent modules should only declare `mod foo;`.

## Documentation Details

When changing user-facing behavior, update both `README.md` and `README_CN.md`.
Keep them conceptually aligned; they do not need to be literal translations.
If release packaging changes, also update `.github/workflows/release.yml` and any matching release-build references in both READMEs.

## Privacy Details

Keep these local and uncommitted:

- `config.toml`
- `captures/`
- `logs/`
- full private request captures
- local repro fixtures containing private prompts

Committed fixtures must be trimmed and sanitized.
