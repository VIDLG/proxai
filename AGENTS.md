# AGENTS.md

Concise guidance for AI agents working in this repository. Detailed lower-frequency notes live in the site docs under `site/src/content/docs/{en,zh}/`.

## Project Intent

`proxai` is a small local compatibility proxy for OpenAI-compatible requests. Keep it focused:

- accept local OpenAI-compatible requests
- normalize the specific Responses API system-message shape that breaks some upstreams
- forward to the configured upstream with minimal surprises
- provide compact diagnostics for real Zed/proxy problems
- evolve toward explicit provider/protocol routing without becoming a generic AI gateway

Do not grow this into a general multi-tenant AI gateway unless explicitly requested.

## Configuration

Keep runtime configuration centered on `config.toml`; `config.example.toml` is the tracked default/example. `config.toml` is intentionally git-ignored.

When adding or changing a runtime setting:

1. Add it to `src/config.rs`.
2. Add it to `config.example.toml` with a concise comment.
3. Wire it through `src/main.rs` and/or `src/lib.rs` if needed.
4. Update `README.md` and `README_CN.md` if user-facing.

Keep CLI flags limited to common temporary overrides (`--config`, `--upstream`, `--api-key`, `--port`, `--log-level`, `--log-format`) unless there is a clear operational need.

## Tooling

Prefer `rtk` wrappers when they materially reduce noisy output, especially for git, tests, builds, diffs, large reads, tree views, and log filtering. Fall back to native commands when clearer.

Good defaults: `rtk git status`, `rtk diff`, `rtk cargo test`, `rtk err cargo test`, `rtk read -l minimal ...`, `rtk tree`.

## Provider / Protocol / Phase Model

Use protocol-based names where wire behavior differs:

- inbound route filtering: `request_protocol`
- outbound provider behavior: provider `protocol`
- provider names: user labels, not semantic protocol identifiers

Current protocol values: `openai_responses`, `openai_chat_completions`, `anthropic_messages`.
If a route omits `request_protocol`, match the actual inbound protocol detected from the request path; provider `protocol` still controls outbound wire behavior. Set `request_protocol` only when the same model pattern needs endpoint-specific routing; model matches with mismatched explicit `request_protocol` should raise a configuration error instead of falling through.

Keep protocol names separate from chain phases:

- `inbound_request`
- `provider_request`
- `upstream_response`
- `outbound_response`

Use phase names for capture artifacts/config, flow locals, and logging fields that describe where data sits in the proxy pipeline.

## Translation Layer

Keep cross-protocol conversion in `src/translation/`:

- `ingress/` owns inbound protocol parsing and normalization.
- `translation/` owns protocol-to-protocol conversion.
- `provider/request` owns provider request preparation, including provider model rewrite, projection/summary extraction, and body serialization.
- `provider/transport` owns target-provider HTTP transport, auth headers, upstream URL construction, and send.
- `http_support` owns HTTP carrier helpers such as response header/body reconstruction and boxed byte streams.

Translation should stay pure at the carrier boundary:

- request translation: `(request_protocol, provider_protocol, normalized_payload) -> payload`
- non-streaming response translation: `(request_protocol, provider_protocol, payload) -> payload`
- streaming response translation: `(request_protocol, provider_protocol, ByteStream) -> ByteStream`

Do not pass HTTP `Response`, `Body`, route/model rewrite details, or provider request structs into `translation/`.

Prefer pair-oriented conversion names such as `openai_responses -> anthropic_messages`. For protocol-specific request/response data, prefer top-level enums keyed by protocol over parallel fields that can drift into impossible states.

When a target protocol cannot represent a source field or block, skip it explicitly with a `tracing::trace!` call that records the discriminant and a short reason. Do not silently drop source-protocol data with `_ => {}` — silent drops make "why did my X disappear" reports unanswerable. "Cannot represent" is not an error: the call site still returns `Ok`, the trace log only makes the drop observable.

## Logging / Errors / Streaming

Logs should be compact, structured, stable, and useful for real debugging. Do not log request bodies, Authorization headers, API keys, private prompts, or unnecessary private upstream URL details.

Keep `error_responses.format = "text"` as the readable default. Preserve useful headers such as `Retry-After`; avoid overfitting to every upstream JSON shape.

Use domain-specific errors rather than broad catch-all conversions:

- `RequestError` for inbound body/validation failures.
- `ConfigError` for config loading and config-file reads.
- `InternalError` for proxy runtime invariants, local filesystem IO, internal HTTP body reads, JSON serialization, and translation boundary errors.
- `UpstreamError` for upstream send/status/body-read failures.
- `TranslationError` for protocol payload conversion.
- `SseError` / `SseTranslationError` for SSE event semantics.
- `ByteStreamError` for stream carrier errors.

Avoid wrapping semantic stream or HTTP errors in `std::io::Error`; reserve `io::Error` for real OS/filesystem IO.

SSE/streaming regressions are user-visible. Preserve SSE bytes and `text/event-stream`, detect terminal events, handle stalled tool-call argument streams, and avoid Unicode chunk slicing panics. Keep semantic tool-call timeout configurable via `[tool_calls].timeout_secs`. Provider streaming internals are documented in `site/src/content/docs/zh/developer/streaming-internals.mdx`; user-facing streaming behavior is in `site/src/content/docs/zh/protocol/streaming-behavior.mdx`.

## Derive Macros

Prefer derive macros over hand-written boilerplate. They keep structural intent visible at a glance and reduce drift between a type and its trait impls.

- `delegate` — `delegate! { to self.field { ... } }` for field delegation instead of one-line wrapper methods. Use it whenever a struct owns a helper type and forwards a subset of its API.
- `derive_more` — `Display`, `From`, `Into` for newtypes and conversions instead of manual `impl` blocks.
- `strum` — `EnumIter`, `EnumString`, `Display` etc. for enums that need iteration or string conversion instead of match-by-match translation tables.
- `getset` — generated accessors when a type needs many `pub` getters/setters with no custom logic, instead of hand-writing them.

Rule of thumb: if a code block only forwards to a field, converts to/from another type, enumerates enum variants, or exposes accessors, it should be a derive. Reserve hand-written `impl` blocks for behavior that has real logic.

## Tests

Favor high-value behavior tests over brittle snapshots/private-helper tests. Important coverage areas:

- `tests/proxy_e2e.rs`
- generated app-directory defaults and config loading
- system-message normalization
- SSE stalls, incomplete tool streams, Unicode stream scanning
- route matching and protocol-aware config behavior

For Rust module tests, prefer adjacent `*_tests.rs` files included from the owning module at the bottom of the implementation file:

```rust
#[cfg(test)]
#[path = "foo_tests.rs"]
mod tests;
```

## Documentation / Privacy

When changing user-facing behavior, update both `README.md` and `README_CN.md`. If release packaging changes, also update `.github/workflows/release.yml` and matching README release-build references.

Documentation follows code and verified behavior; code does not follow aspirational docs. When docs and implementation disagree, inspect the implementation, tests, and config defaults first, then update docs or code deliberately rather than treating the docs as the source of truth.

Keep local/private artifacts uncommitted: `config.toml`, `captures/`, `logs/`, full private captures, and local repro fixtures containing private prompts. Committed fixtures must be trimmed and sanitized.

## Validation

Preferred checks:

- `just check` for the normal full local validation path
- `cargo fmt --check` for formatting-only checks
- `pixi run cargo clippy --lib --tests -- -D warnings` for warning-free Rust checks
- `pixi run cargo test --lib` for quick unit coverage
- `just test-e2e` when changing proxy behavior, SSE handling, capture behavior, or request normalization
- `just probe-model-limits --models gpt-5.4,gpt-5.5,gpt-5.3-codex` when practical upstream Responses API limits must be measured

On Windows, prefer the `just` test recipes over direct `pixi run cargo test` when a local proxy may be running. `just test`, `just test_lib`, and `just test-e2e` set `CARGO_TARGET_DIR=.cargo-target-tests`, which avoids trying to overwrite a locked `target/debug/proxai.exe`. Direct `pixi run cargo test` can fail if `target/debug/proxai.exe` is locked by a running proxy; stop it or use the matching `just` recipe.
