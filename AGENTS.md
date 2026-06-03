# AGENTS.md

Concise guidance for AI agents working in this repository. Detailed lower-frequency notes live in `docs/agent-guidelines.md`.

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
- `provider/` owns target-provider transport and provider-local request/response behavior.

Prefer pair-oriented conversion names such as `openai_responses -> anthropic_messages`. For protocol-specific request/response data, prefer top-level enums keyed by protocol over parallel fields that can drift into impossible states.

## Logging / Errors / Streaming

Logs should be compact, structured, stable, and useful for real debugging. Do not log request bodies, Authorization headers, API keys, private prompts, or unnecessary private upstream URL details.

Keep `error_responses.format = "text"` as the readable default. Preserve useful headers such as `Retry-After`; avoid overfitting to every upstream JSON shape.

SSE/streaming regressions are user-visible. Preserve SSE bytes and `text/event-stream`, detect terminal events, handle stalled tool-call argument streams, and avoid Unicode chunk slicing panics. Keep semantic tool-call timeout configurable via `[tool_calls].timeout_secs`.

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

Keep local/private artifacts uncommitted: `config.toml`, `captures/`, `logs/`, full private captures, and local repro fixtures containing private prompts. Committed fixtures must be trimmed and sanitized.

## Validation

Preferred checks:

- `just check` for the normal full local validation path
- `cargo fmt --check` for formatting-only checks
- `pixi run cargo clippy --lib --tests -- -D warnings` for warning-free Rust checks
- `pixi run cargo test --lib` for quick unit coverage
- `just test-e2e` when changing proxy behavior, SSE handling, capture behavior, or request normalization
- `just probe-model-limits --models gpt-5.4,gpt-5.5,gpt-5.3-codex` when practical upstream Responses API limits must be measured

On Windows, `pixi run cargo test` can fail if `target/debug/proxai.exe` is locked by a running proxy; stop it and retry.
