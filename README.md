# ProxAI

📚 **Docs site**: [vidlg.github.io/proxai](https://vidlg.github.io/proxai) (Astro / Starlight, source in `site/`)

[中文文档](README_CN.md)

ProxAI is a small local compatibility proxy for OpenAI-compatible requests.
It accepts local client traffic, normalizes the narrow Zed-specific OpenAI
Responses system-message and compact history shapes that break some upstreams,
and forwards requests to the configured provider with minimal surprises.

Today, the stable runtime paths support no-conversion forwarding for OpenAI
Responses, OpenAI Chat Completions, and Anthropic Messages, plus explicit
cross-protocol translation for selected protocol pairs. The config model is
protocol-aware so routing and conversion paths can expand explicitly over time
without turning ProxAI into a generic AI gateway.

## Current Status

The current stable forwarding and translation paths are:

- inbound: `openai_responses` → outbound: `openai_responses`
- inbound: `openai_chat_completions` → outbound: `openai_chat_completions`
- inbound: `anthropic_messages` → outbound: `anthropic_messages`
- inbound: `openai_responses` → outbound: `openai_chat_completions`
- inbound: `openai_responses` → outbound: `anthropic_messages`
- inbound: `openai_chat_completions` → outbound: `anthropic_messages`
- inbound: `openai_chat_completions` → outbound: `openai_responses`
- inbound: `anthropic_messages` → outbound: `openai_responses`
- inbound: `anthropic_messages` → outbound: `openai_chat_completions`

See [Protocols Reference](https://vidlg.github.io/proxai/reference/protocols) for the full matrix and pair-specific lossiness.

For Chat-compatible clients such as Zed, plain reasoning text is preserved through
the `reasoning_content` extension in assistant history, non-streaming messages,
and streaming deltas. Zed also accepts `reasoning` in streaming deltas, while it
replays assistant history as `reasoning_content`. These compatibility fields are
injected and extracted at the translation boundary; the official OpenAI Chat wire
types remain aligned with the OpenAPI schema. Zed Responses request replay is
normalized separately before strict parsing: ProxAI completes compact assistant
output envelopes, id-less reasoning summary items, and omitted message-image
`detail` defaults while keeping the official Responses wire types unchanged. In
provider compatibility mode,
ProxAI also repairs measured upstream omissions such as MiniMax Chat streaming
chunks without required-nullable `choices[].finish_reason`. It also accepts
Bedrock Mantle's coordinate-optional `response.reasoning.delta/done` events
without inventing Responses item identities. Redacted or encrypted reasoning is
never exposed as ordinary visible content.

When translating Anthropic thinking across turns, ProxAI uses a versioned,
client-carried continuation envelope: Responses uses `reasoning.encrypted_content`,
and Chat-compatible history uses a suffix in `reasoning_content`. ProxAI removes
that envelope and restores the provider-specific thinking blocks before forwarding
the next request to Anthropic; it keeps no proxy-side continuation state.

Request and stream translation failures automatically create bounded local diagnostic
bundles under `diagnostics/`, independent of capture. Stream bundles retain the raw
triggering SSE frame locally; normal logs expose only safe context and `diag=...`.

## Quick Start

1. Download the Windows release executable, or build from source.
2. Run ProxAI once to generate the app directory and `config.example.toml`.
3. Edit `config.toml` (under `%USERPROFILE%\.proxai\` on Windows,
   `~/.proxai/` on Linux/macOS) to set provider `base_url` and `api_key`.
4. Point your OpenAI-compatible client at `http://127.0.0.1:18080/v1`.

For the full walkthrough, see [Quick Start](https://vidlg.github.io/proxai/using/quick-start).

## Default Endpoints

| Endpoint | Default URL |
|---|---|
| Proxy | `http://127.0.0.1:18080` |
| MCP | `http://127.0.0.1:18081/mcp` |

For all other defaults and limits, see [Defaults and Limits](https://vidlg.github.io/proxai/reference/defaults-and-limits).

## CLI

CLI flags are intentionally small and used for temporary overrides only:

```sh
proxai --config <path> \
       --upstream <url> \
       --api-key <key> \
       --port <port> \
       --log-level <level> \
       --log-format <human|json> \
       --route-override ROUTE.FIELD=VALUE
```

For the full reference (including the `capture` subcommand), see [CLI Reference](https://vidlg.github.io/proxai/reference/cli).

## Documentation

The complete documentation lives in `site/src/content/docs/` and is published to
[vidlg.github.io/proxai](https://vidlg.github.io/proxai). Key sections:

- [Using ProxAI](https://vidlg.github.io/proxai/using) — user-facing task guide
- [Configuration](https://vidlg.github.io/proxai/using/configuration) — runtime settings, routes, providers, capture, logging, errors
- [Routing and Providers](https://vidlg.github.io/proxai/using/routing-and-providers) — how providers are selected
- [Observability](https://vidlg.github.io/proxai/using/observability) — compact logs, automatic failure diagnostics, capture, and privacy boundaries
- [Troubleshooting](https://vidlg.github.io/proxai/using/troubleshooting) — common symptoms and next checks
- [Protocol Overview](https://vidlg.github.io/proxai/protocol) — phase axis, protocol axis, conversion matrix
- [Streaming Behavior](https://vidlg.github.io/proxai/protocol/streaming-behavior) — terminal events, tool-call timeouts
- [Architecture](https://vidlg.github.io/proxai/developer/architecture) — request lifecycle, module boundaries
- [Behavior Contracts](https://vidlg.github.io/proxai/reference/behavior-contracts) — stable promises ProxAI commits to

Reference pages:

- [Configuration Reference](https://vidlg.github.io/proxai/reference/configuration) — full `config.example.toml`
- [CLI](https://vidlg.github.io/proxai/reference/cli) — runtime flags and capture subcommands
- [Defaults and Limits](https://vidlg.github.io/proxai/reference/defaults-and-limits)
- [Protocols](https://vidlg.github.io/proxai/reference/protocols) — values, paths, conversion pairs
- [Route Matching](https://vidlg.github.io/proxai/reference/route-matching) — route outcomes, protocol guards, and fallback behavior
- [Capture Phases](https://vidlg.github.io/proxai/reference/capture-phases) — capture boundaries and privacy risk
- [Environment and Files](https://vidlg.github.io/proxai/reference/environment-and-files) — app directories and local artifacts
- [Error Responses](https://vidlg.github.io/proxai/reference/error-responses) — payload, type enum, HTTP status
- [Glossary](https://vidlg.github.io/proxai/reference/glossary) — shared terminology

## Development

Common commands:

- `pixi install`
- `just run` — run ProxAI locally
- `just check` — full local validation, including the OpenAI protocol drift check
- `just test-e2e` — end-to-end tests
- `just build` — release build
- `cargo run -- check-update` — check for updates

Protocol coverage comparison against official protocol references:

- `just protocol-compare` — OpenAI schema required-field drift gate used by `just check`
- `just compare-openai-protocol` — detailed required-field check against the official OpenAPI schema; pass `--structural` for the manual schema-first audit of unmapped wire types and unmodeled properties (not part of CI)
- `just compare-anthropic-protocol` — Anthropic Messages types vs official TS SDK

The referenced protocol checkouts are git submodules under `contrib/`:

- `contrib/openai-openapi`
- `contrib/anthropic-sdk-typescript`

For the alignment rules enforced by these scripts, see
[Protocol Conversion](https://vidlg.github.io/proxai/developer/protocol-conversion).

## Documentation Site

The docs site is built with Astro + Starlight. From the repository root:

```sh
just site install   # install dependencies (pnpm via pixi)
just site dev       # local dev server at http://localhost:4321
just site build     # production build into site/dist
just site check     # build + docs i18n/structure validation
```

See [`site/README.md`](site/README.md) for details.

## Release Artifacts

GitHub release artifacts are versioned like:

- `proxai-vX.Y.Z-windows-x86_64.exe`

## Notes on Future Protocols

The current repo keeps cross-protocol translation and route-level protocol
filtering explicit. Add new protocol pairs deliberately, with runtime routing,
request/response conversion, and tests for the exact pair, rather than growing
ProxAI into a generic AI platform by accident.
