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
If a route omits `request_protocol`, match the actual inbound protocol detected from the request path; provider `protocol` still controls outbound wire behavior. Set `request_protocol` only when the same model pattern needs endpoint-specific routing. A model match with a mismatched explicit `request_protocol` may continue to a later model match whose protocol is compatible; if no compatible route exists, raise a configuration error instead of falling through to the default provider.

Keep protocol names separate from chain phases:

- `inbound_request`
- `provider_request`
- `upstream_response`
- `outbound_response`

Use phase names for capture artifacts/config, flow locals, and logging fields that describe where data sits in the proxy pipeline.

## Translation Layer

Keep cross-protocol conversion in `crates/proxai-core/src/translation/`:

- `src/pipeline/inbound.rs` owns inbound path detection, JSON byte parsing, application error mapping, and request-scoped observer wiring.
- `crates/proxai-core/src/ingress/` owns structured inbound normalization and validation.
- `crates/proxai-core/src/observe.rs` owns the shared core observation contract and typed variants.
- `crates/proxai-core/src/protocol/` owns wire models.
- `crates/proxai-core/src/routing/` owns carrier-independent route configuration, matcher compilation, provider-label selection, upstream model rewrite, and typed routing errors.
- `crates/proxai-core/src/translation/` owns protocol-to-protocol conversion.
- `crates/proxai-core/src/provider/` owns carrier-independent provider compatibility policy, structured response normalization, and typed provider adaptation observations.
- `provider/request` owns application-side provider request preparation, including provider model rewrite, projection/summary extraction, and body serialization.
- `provider/transport` owns target-provider HTTP transport, auth headers, upstream URL construction, and send.
- `http_support` owns HTTP carrier helpers such as response header/body reconstruction and boxed byte streams.

Core ingress, routing, and translation should stay pure at the carrier boundary:

- inbound preparation: `(request_protocol, payload) -> prepared_request`
- route resolution: `(routing_config, provider_names, request_protocol, model) -> resolved_route`
- request translation: `(request_protocol, provider_protocol, normalized_payload) -> payload`
- non-streaming response translation: `(request_protocol, provider_protocol, payload) -> payload`
- streaming response translation: `(request_protocol, provider_protocol, Stream<StreamTranslationInput>) -> Stream<StreamEvent>`

Do not pass HTTP `Response`, `Body`, `ByteStream`, SSE frames, route/model rewrite details, or provider request structs into core translation.

Prefer pair-oriented conversion names such as `openai_responses -> anthropic_messages`. For protocol-specific request/response data, prefer top-level enums keyed by protocol over parallel fields that can drift into impossible states.

When a target protocol cannot represent a source field or block, report a typed translation observation through the core `Observer` with the discriminant and a short reason. Do not silently drop source-protocol data with `_ => {}` — silent drops make "why did my X disappear" reports unanswerable. "Cannot represent" is not an error: the call site still returns `Ok`; the downstream observer decides how to log or diagnose the loss.

## Logging / Errors / Streaming

Logs should be compact, structured, stable, and useful for real debugging. Do not log request bodies, Authorization headers, API keys, private prompts, or unnecessary private upstream URL details.

Core crates may define observation traits, closed structured observation variants, and no-op defaults, but must not choose concrete logging, diagnostics, capture, metrics, or storage implementations. Downstream composition supplies those implementations.

Core request-domain producers emit typed `Observation` values through `Observer`; downstream `ObserveContext` implements that contract, and its sinks decide logging level/format, diagnostics, and capture. Use `tracing` directly only inside logging/observation sinks, for observation-system failures that must not recurse, for process-level startup/config/background events without a request context, or for genuinely temporary local debugging. If a trace becomes a recurring diagnostic signal, promote it to an observe point.

Keep `error_responses.format = "text"` as the readable default. Preserve useful headers such as `Retry-After`; avoid overfitting to every upstream JSON shape.

Use domain-specific errors rather than broad catch-all conversions:

- `JsonPayloadError` for shared core JSON path/source details; ingress and translation wrap it transparently.
- `IngressError` for carrier-independent structured normalization and validation failures.
- `RequestError` for application HTTP body/path failures and transparent wrapping of `IngressError`.
- `ConfigError` for config loading and config-file reads.
- `InternalError` for proxy runtime invariants, local filesystem IO, internal HTTP body reads, JSON serialization, and translation boundary errors.
- `UpstreamError` for upstream send/status/body-read failures.
- `TranslationError` for non-streaming protocol payload conversion.
- `StreamTranslationError` for structured stream conversion and lifecycle semantics.
- `SseError` for application SSE carrier parsing.
- `ByteStreamError` for byte stream carrier errors.

Core errors return through `Result`; non-fatal adaptations emit typed `Observation` values. Do not emit an observation for the same failure that is returned as an error—the downstream boundary observes and renders that error once with carrier context.

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

### Real-world regression tests

Distinguish tests that **synthesize a payload to exercise a code branch** from tests that **reproduce a real-world failure observed in production / dogfooding / upstream protocol drift**. The second category is far more valuable: it is the project's memory of concretely observed bugs and stays in the tree forever.

Mark a real-world regression test with three things:

1. **`regression_<source>_<symptom>` name prefix** — so `grep regression_` lists every real-data regression in the tree.
   - `<source>`: the upstream or client that triggered it (`zed_`, `glm_`, `anthropic_`, `opus_`, ...).
   - `<symptom>`: short description of what failed (`reasoning_dropped`, `tool_call_stall`, `unicode_panic`, ...).
   - Example: `regression_glm_interleaved_reasoning_dropped_before_assistant_turn`.
2. **A source comment** directly above `#[test]` stating:
   - the trigger condition (what payload / upstream behavior caused it),
   - the observed symptom (panic / silent drop / wrong shape / stall),
   - the data provenance (`capture 2025-06-15 req_7f3a...`, `Zed 0.180.0`, `GLM Responses 2025-Q2`) and a sanitization note when the original prompt was redacted.
3. **Fixture file** when the real payload is large (>~30 lines JSON / multi-event SSE):
   - committed under `tests/fixtures/regression/`, named in correspondence with the test (`regression_glm_reasoning_dropped` ↔ `glm-reasoning-interleaved-request.json`).
   - must be sanitized (no API keys, no private prompt text) — this is already required by the privacy rules below.

Synthetic tests keep their normal names (`translates_xxx`, `rejects_xxx`, ...). Do not retroactively rename them to `regression_*` unless the payload genuinely came from an observed failure.

**Isolate regression tests into a separate `*_regression_tests.rs` file** adjacent to the regular test file. The two test populations have very different value density and lifecycle:

- Synthetic tests are easy to write and grow fast (AI assistants in particular generate them quickly); mixing them with real-world regressions buries the rare, high-value cases.
- Regression tests are scarce, carry project memory, and must survive refactors. Physical isolation makes that scarcity visible at a glance — a module with `foo_tests.rs` (60 synthetic cases) + `foo_regression_tests.rs` (3 real regressions) tells you immediately where the project's hard-won knowledge lives.

Recommended layout for a module that owns both kinds of tests:

```rust
// crates/proxai-core/src/translation/foo.rs
#[cfg(test)]
#[path = "foo_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "foo_regression_tests.rs"]
mod regression_tests;
```

Both modules still discover via `#[cfg(test)]`, so `cargo test`, `grep regression_`, and `just regression-run` work uniformly across the tree.

A regression test is a contract — **do not delete or relax its assertions when refactoring the surrounding code**. If the original behavior was actually wrong, replace the regression test with a new one that captures the corrected behavior, preserving the source comment trail.

### Failure trust gradient (TDD inverted)

Tests are not equally trustworthy when they fail. A test's value density decides how much a failure should be trusted — and the gradient runs the opposite of what naive TDD assumes.

**`regression_*` test fails → assume the code is broken.** The payload is real (observed in production / dogfooding / upstream drift), the assertions lock a concrete past fix, and the test carries project memory. Treat any failure as a real regression until proven otherwise. Resolve by fixing the code. Only edit the test if the original behavior was genuinely wrong, and even then replace it with a new regression test that preserves the source comment trail.

**Synthetic `*_tests` fails → assume the test may itself be wrong.** The payload was handcrafted to hit a branch, the assertions encode a developer's assumption about target shape, and AI-generated tests in particular tend to over-couple to implementation details. Three equally plausible causes:
1. real code regression (medium likelihood),
2. brittle test coupled to refactored internals (high likelihood for AI-written tests),
3. the synthesized payload never matched real upstream shape (medium likelihood).

Default action for synthetic test failures: investigate briefly, then either fix the code or rewrite/delete the test without ceremony. Do not contort production code to satisfy a brittle synthetic test.

This is why physical file separation matters for review attention and CI triage: when a build breaks, `*_regression_tests.rs` failing is a high-priority signal that demands investigation, while `*_tests.rs` failing is a lower-priority signal that may just need the test rewritten. Mixing them flattens the trust gradient and drowns the rare high-confidence failures in noise from brittle synthetic tests.

Recommended local recipes:

- `just regression-run` runs every `regression_*` test (see `justfile`).
- `just regression-list` lists them by name.
- `just regression-touched` shows which regression test files a working diff touches — use it during review to focus attention.



## Documentation / Privacy

When changing user-facing behavior, update both `README.md` and `README_CN.md`. If release packaging changes, also update `.github/workflows/release.yml` and matching README release-build references.

Documentation follows code and verified behavior; code does not follow aspirational docs. When docs and implementation disagree, inspect the implementation, tests, and config defaults first, then update docs or code deliberately rather than treating the docs as the source of truth.

Keep local/private artifacts uncommitted: `config.toml`, `captures/`, `logs/`, full private captures, and local repro fixtures containing private prompts. Committed fixtures must be trimmed and sanitized.

## Validation

Preferred checks:

- `just check` for the normal full local validation path
- `cargo fmt --all --check` for formatting-only checks
- `pixi run cargo clippy --workspace --all-targets -- -D warnings` for warning-free Rust checks
- `pixi run cargo test --workspace --lib` for quick unit coverage
- `just test-e2e` when changing proxy behavior, SSE handling, capture behavior, or request normalization
- `just probe-model-limits --models gpt-5.4,gpt-5.5,gpt-5.3-codex` when practical upstream Responses API limits must be measured

On Windows, prefer the `just` test recipes over direct `pixi run cargo test` when a local proxy may be running. `just test`, `just test_lib`, and `just test-e2e` set `CARGO_TARGET_DIR=.cargo-target-tests`, which avoids trying to overwrite a locked `target/debug/proxai.exe`. Direct `pixi run cargo test` can fail if `target/debug/proxai.exe` is locked by a running proxy; stop it or use the matching `just` recipe.
