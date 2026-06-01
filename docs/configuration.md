# Configuration

This document explains the runtime configuration model used by ProxAI.

## Scope and current runtime status

ProxAI uses a provider / routing configuration model with these stable no-conversion forwarding paths today:

- inbound: `openai_responses` -> outbound: `openai_responses`
- inbound: `openai_chat_completions` -> outbound: `openai_chat_completions`
- inbound: `anthropic_messages` -> outbound: `anthropic_messages`

Cross-protocol conversion remains explicit and is not enabled implicitly by defaults.

## App directory

Generated runtime files live under the user app directory:

- Windows: `%USERPROFILE%\\.proxai\\`
- Linux/macOS: `~/.proxai/`

Important files and folders:

- `config.toml`
- `config.example.toml`
- `logs/`
- `captures/`

## `[server]`

Controls the local listen host and port.

## `[mcp]`

Configures the local listen host and port for the MCP control/API listener.

Fields:

- `host`
- `port`

ProxAI currently starts a local streamable HTTP MCP endpoint at `/mcp` on this address.
With the default config, that means:

- `http://127.0.0.1:18081/mcp`

## `[routing.default_provider_names]`

Declares fallback providers per inbound request protocol.

Current keys:

- `openai_responses`
- `openai_chat_completions`
- `anthropic_messages`

These values are used only when no explicit route matches.

## `[[routing.routes]]`

Routes are filters over inbound requests.

Fields:

- `request_protocol` optional
- `match_kind` optional
- `model_pattern`
- `provider_name`
- `upstream_model` optional

### `request_protocol`

This is a route filter, not an input from the client.

At runtime, ProxAI detects the inbound request protocol from request shape and path. A route then uses `request_protocol` to say whether it applies to that detected protocol.

If `request_protocol` is omitted, it defaults to the selected provider's `protocol`, i.e. the no-conversion path.

Cross-protocol routing should therefore be explicit.

### `match_kind`

Supported values:

- `exact`
- `glob`
- `regex`
- `auto`

If omitted, it defaults to `auto`.

### `model_pattern`

The logical model selector used for route matching.

Examples:

- `gpt-5.4`
- `gpt-*`
- `^claude-(?<tier>.+)$`

### `provider_name`

The provider name selected when this route matches.

### `upstream_model`

Optional upstream model mapping.

Behavior:

- omitted: forward the original request model unchanged
- `exact` / `glob`: treated as a fixed upstream model string
- `regex`: treated as a regex replacement template, supporting `$1` or `$name`

## `[providers.<name>]`

Each provider describes how ProxAI talks to a specific upstream.
All fields below are required.

Fields:

- `protocol`
- `base_url`
- `api_key`
- `compatibility` optional
- `read_idle_timeout_secs`

Current protocol values:

- `openai_responses`
- `openai_chat_completions`
- `anthropic_messages`

### Provider API key override behavior

Provider `api_key` is required.
For OpenAI providers (`openai_responses` and `openai_chat_completions`), ProxAI sends that key upstream as `Authorization: Bearer <key>` and ignores any `Authorization` header received from the client. For Anthropic Messages providers, ProxAI sends `x-api-key`.

This allows Zed to use a dummy key when required by the UI while ProxAI supplies the real upstream key.

### `compatibility`

Supported values:

- `strict`
- `anthropic_compatible`

For `anthropic_messages` providers, `anthropic_compatible` may fill compatibility
gaps before ProxAI logs, translates, or returns successful responses. Today it
normalizes SDK required-nullable response fields that are missing by inserting
explicit `null` values, normalizes bare `message_start` events into the
official nested `message` shape, repairs the measured MiniMax case where a
streamed thinking `content_block_start` omits `signature`, and repairs the
measured GLM 5.1 case where `server_tool_use` includes only one counter.

Do not add default values for other provider-specific missing fields, such as
missing tool callers, unless they are backed by a measured upstream case and a
focused fixture.

Use `strict` for the official Anthropic API or any upstream that already emits
the official Messages schema. If omitted, the default is
`anthropic_compatible` for local compatibility.

## `[tool_calls]`

`timeout_secs` is the semantic timeout for incomplete streamed tool-call arguments.

It is always enabled and must be greater than zero.

## `[capture]`

- `inbound_request_enabled = true` writes the client request as proxai received it to the predefined app-dir `captures/` folder.
- `inbound_request_enabled = false` disables inbound request capture output.
- `forwarded_request_enabled = true` writes the request proxai actually forwards upstream after adaptation.
- `forwarded_request_enabled = false` disables forwarded request capture output.
- `upstream_response_enabled = true` writes upstream response headers and raw upstream response bytes.
- `upstream_response_enabled = false` disables upstream response capture output.
- `outbound_response_enabled = true` writes the final response proxai sends back to the client.
- `outbound_response_enabled = false` disables outbound response capture output.

Capture paths are not configurable. ProxAI always prepares the app-dir
`captures/` folder at startup; the phase switches only control whether a
request writes artifacts there.

These are runtime defaults. For persistent local changes, the CLI supports:

- `proxai capture status`
- `proxai capture enable [inbound-request|forwarded-request|upstream-response|outbound-response]`
- `proxai capture disable [inbound-request|forwarded-request|upstream-response|outbound-response]`

For temporary debugging, run-time flags can enable any capture phase for a single process invocation.

## `[logging]` and `[logging.duration_thresholds]`

- `output_format = "human"` for compact interactive debugging output
- `output_format = "json"` for machine consumption
- `use_color = true` to enable colored human logs
- `use_color = false` to disable colored human logs
- `warn_ms` / `error_ms` for human log coloring thresholds

## `[error_responses]`

- `text` for concise Zed-readable error bodies
- `json` for OpenAI-style error bodies for non-Zed clients

For upstream non-2xx responses, ProxAI normalizes the response body and preserves
useful diagnostic headers such as `Retry-After`, upstream request ids, and
rate-limit headers.

## Timeout semantics

### `read_idle_timeout_secs`

This is not a total request duration cap.
It is a per-provider read-idle timeout that resets whenever upstream bytes arrive.

### `[tool_calls].timeout_secs`

This is a semantic timeout for incomplete streamed tool-call argument flows.
It exists specifically to stop clients from hanging forever when an upstream starts tool arguments but never finishes them.
