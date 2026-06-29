# ProxAI

📚 **Docs site**: [vidlg-proxai.netlify.app](https://vidlg-proxai.netlify.app) (Astro / Starlight, source in `site/`)

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

- inbound: `openai_responses` → outbound: `openai_responses`
- inbound: `openai_chat_completions` → outbound: `openai_chat_completions`
- inbound: `anthropic_messages` → outbound: `anthropic_messages`
- inbound: `openai_responses` → outbound: `openai_chat_completions`
- inbound: `openai_responses` → outbound: `anthropic_messages`
- inbound: `openai_chat_completions` → outbound: `anthropic_messages`
- inbound: `anthropic_messages` → outbound: `openai_responses`

Other cross-protocol translation paths remain intentionally unsupported until
they are implemented explicitly. See [Protocols Reference](https://vidlg-proxai.netlify.app/en/reference/protocols)
for the full matrix.

## Quick Start

1. Download the Windows release executable, or build from source.
2. Run ProxAI once to generate the app directory and `config.example.toml`.
3. Edit `config.toml` (under `%USERPROFILE%\.proxai\` on Windows,
   `~/.proxai/` on Linux/macOS) to set provider `base_url` and `api_key`.
4. Point your OpenAI-compatible client at `http://127.0.0.1:18080/v1`.

For the full walkthrough, see [Quick Start](https://vidlg-proxai.netlify.app/en/using/quick-start).

## Default Endpoints

| Endpoint | Default URL |
|---|---|
| Proxy | `http://127.0.0.1:18080` |
| MCP | `http://127.0.0.1:18081/mcp` |

For all other defaults and limits, see [Defaults and Limits](https://vidlg-proxai.netlify.app/en/reference/defaults-and-limits).

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

For the full reference (including the `capture` subcommand), see [CLI Reference](https://vidlg-proxai.netlify.app/en/reference/cli).

## Documentation

The complete documentation lives in `site/src/content/docs/` and is published to
[vidlg-proxai.netlify.app](https://vidlg-proxai.netlify.app). Key sections:

- [Using ProxAI](https://vidlg-proxai.netlify.app/en/using) — user-facing task guide
- [Configuration](https://vidlg-proxai.netlify.app/en/using/configuration) — runtime settings, routes, providers, capture, logging, errors
- [Routing and Providers](https://vidlg-proxai.netlify.app/en/using/routing-and-providers) — how providers are selected
- [Observability](https://vidlg-proxai.netlify.app/en/using/observability) — capture, logs, privacy boundaries
- [Troubleshooting](https://vidlg-proxai.netlify.app/en/using/troubleshooting) — common symptoms and next checks
- [Protocol Overview](https://vidlg-proxai.netlify.app/en/protocol) — phase axis, protocol axis, conversion matrix
- [Streaming Behavior](https://vidlg-proxai.netlify.app/en/protocol/streaming-behavior) — terminal events, tool-call timeouts
- [Architecture](https://vidlg-proxai.netlify.app/en/developer/architecture) — request lifecycle, module boundaries
- [Behavior Contracts](https://vidlg-proxai.netlify.app/en/reference/behavior-contracts) — stable promises ProxAI commits to

Reference pages:

- [Configuration Reference](https://vidlg-proxai.netlify.app/en/reference/configuration) — full `config.example.toml`
- [CLI](https://vidlg-proxai.netlify.app/en/reference/cli) — runtime flags and capture subcommands
- [Defaults and Limits](https://vidlg-proxai.netlify.app/en/reference/defaults-and-limits)
- [Protocols](https://vidlg-proxai.netlify.app/en/reference/protocols) — values, paths, conversion pairs
- [Route Matching](https://vidlg-proxai.netlify.app/en/reference/route-matching) — route outcomes, protocol guards, and fallback behavior
- [Capture Phases](https://vidlg-proxai.netlify.app/en/reference/capture-phases) — capture boundaries and privacy risk
- [Environment and Files](https://vidlg-proxai.netlify.app/en/reference/environment-and-files) — app directories and local artifacts
- [Error Responses](https://vidlg-proxai.netlify.app/en/reference/error-responses) — payload, type enum, HTTP status
- [Glossary](https://vidlg-proxai.netlify.app/en/reference/glossary) — shared terminology

## Development

Common commands:

- `pixi install`
- `just run` — run ProxAI locally
- `just check` — full local validation
- `just test-e2e` — end-to-end tests
- `just build` — release build
- `cargo run -- check-update` — check for updates

Protocol coverage comparison against official SDKs:

- `just compare-anthropic-protocol` — Anthropic Messages types vs official TS SDK
- `just compare-openai-protocol` — OpenAI types vs `async-openai` v0.40.2

The referenced SDK checkouts are git submodules under `contrib/`:

- `contrib/anthropic-sdk-typescript`
- `contrib/async-openai`

For the alignment rules enforced by these scripts, see
[Protocol Conversion](https://vidlg-proxai.netlify.app/en/developer/protocol-conversion).

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
