# Zed OpenAI Shim

[中文文档](README_CN.md)

Local proxy for using Zed's OpenAI-compatible Responses API client with an
upstream that rejects `role:"system"` messages in `input`.

The shim listens on `127.0.0.1:18080` by default, forwards requests to the
configured upstream, and only normalizes the system-message shape that breaks
the upstream.

## Overview

- Users: download the release zip, place `.env` next to
  `zed-openai-shim.exe`, then run the exe.
- Developers: use `pixi install` and `just run` for local iteration.
- Zed: configure an `openai_compatible` provider whose `api_url` is
  `http://127.0.0.1:18080/v1`.

## Problem

Zed sends agent requests to `/v1/responses` with a Responses-style `input`
array. Agent instructions are sent as a system message:

```json
{
  "type": "message",
  "role": "system",
  "content": [
    {
      "type": "input_text",
      "text": "You are a coding agent."
    }
  ]
}
```

Some upstreams, including the sub2api OpenAI OAuth Codex path this shim was
built for, reject system messages in `input` and return an error like:

```json
{
  "detail": "System messages are not allowed"
}
```

The compatible shape is to move the system text to top-level `instructions` and
remove the system item from `input`. See the upstream sub2api issue
[Wei-Shaw/sub2api#2147](https://github.com/Wei-Shaw/sub2api/issues/2147) and
[docs/sub2api-system-role-issue.md](docs/sub2api-system-role-issue.md) for the
root-cause notes.

## What The Shim Changes

For JSON requests, the shim:

- finds top-level `input` array items with `role:"system"`
- extracts text from `content` parts with `type:"input_text"` or `type:"text"`
- prepends that text to top-level `instructions`
- removes the original system message from `input`
- leaves other fields, headers, tools, `prompt_cache_key`, and request paths as
  unchanged as possible

Example normalized request:

```json
{
  "instructions": "You are a coding agent.",
  "input": [
    {
      "type": "message",
      "role": "user",
      "content": [
        {
          "type": "input_text",
          "text": "Hello"
        }
      ]
    }
  ]
}
```

The shim does not convert Chat Completions payloads. The current scope is the
Zed Responses API role issue.

## User Setup

Download `zed-openai-shim-windows-x86_64.zip` from GitHub Releases, unzip it,
then copy `.env.example` to `.env`. For a portable release folder, keep `.env`
next to `zed-openai-shim.exe`:

```text
zed-openai-shim/
  zed-openai-shim.exe
  .env.example
  .env
```

The exe loads configuration from these `.env` locations, in order:

1. `.env` next to `zed-openai-shim.exe`
2. `%USERPROFILE%\.zed-openai-shim\.env`
3. `.env` in the current working directory

Values loaded earlier win because `dotenvy` does not override existing
environment variables. Use the exe-local `.env` for portable folders, or the
home-directory `.env` if you want to download/update the exe without moving a
config file beside it each time.

Create `.env` from the example:

```powershell
Copy-Item .env.example .env
```

Or create the home-directory config once:

```powershell
New-Item -ItemType Directory -Force "$env:USERPROFILE\.zed-openai-shim"
Copy-Item .env.example "$env:USERPROFILE\.zed-openai-shim\.env"
```

Then edit `.env` and set your upstream:

```powershell
OPENAI_SHIM_UPSTREAM=http://your-upstream.example:8080
```

Run the exe:

```powershell
.\zed-openai-shim.exe
```

By default, the shim listens on `http://127.0.0.1:18080` and forwards to
`OPENAI_SHIM_UPSTREAM` from `.env`.

If Zed does not send an `Authorization` header, the shim can add one from
`.env`:

```powershell
OPENAI_SHIM_API_KEY=replace-with-your-api-key
```

If Zed requires a key and sends a dummy `Authorization` header, enable explicit
override in `.env`:

```powershell
OPENAI_SHIM_OVERRIDE_AUTHORIZATION=true
```

Do not commit real API keys.

## Developer Workflow

Install the Pixi-managed GNU toolchain and run from source:

```powershell
pixi install
just run
```

Build a local release exe:

```powershell
just build
```

The local build output is `target\release\zed-openai-shim.exe`.

Common commands:

| Command | Description |
| --- | --- |
| `pixi install` | Install the Pixi-managed GNU toolchain used by the Rust build. |
| `just run` | Run the shim from source for development. |
| `just check` | Run formatting, clippy, and Rust tests. |
| `just test-e2e` | Run the Rust proxy E2E test against a fake upstream. |
| `just repro-system-role` | Optional real-upstream diagnostic for the system-role behavior. |
| `just build` | Build `target/release/zed-openai-shim.exe`. |

`just repro-system-role` is intentionally not part of `just check`: it calls the
configured real upstream and requires `OPENAI_SHIM_API_KEY`.

## Release Build

GitHub Actions uses the same Pixi + Just path as local development:
`just check` and `just build`. The Rust toolchain is
`stable-x86_64-pc-windows-gnu`, and Pixi provides the GNU linker/toolchain.

The release workflow runs on manual dispatch and on version tags such as
`v0.1.1`. Tagged runs publish:

```text
zed-openai-shim-windows-x86_64.zip
```

The zip contains `zed-openai-shim.exe` and `.env.example`. Copy `.env.example`
to `.env`, edit it, and keep `.env` beside the exe.

Release notes are generated in two steps:

1. `git-cliff` builds deterministic notes from commits since the previous tag.
2. `scripts/polish_release_notes.py` optionally uses Anthropic-compatible
   credentials to rewrite those notes into a short GitHub Release body.

If `ANTHROPIC_BASE_URL` and `ANTHROPIC_API_KEY` are not configured, release
notes fall back to the raw `git-cliff` output. The release still succeeds.

Local release-notes preview:

```powershell
pixi run git-cliff --latest --output dist\release-notes.raw.md
pixi run python scripts\polish_release_notes.py --input dist\release-notes.raw.md --output dist\release-notes.md
```

## Zed Configuration

In Zed settings, configure an OpenAI-compatible provider that points at the
local shim. Do not put API keys in `settings.json`; Zed stores provider keys in
the OS keychain when entered through the Agent settings UI, and also supports
provider-specific environment variables.

```json
{
  "language_models": {
    "openai_compatible": {
      "openai_shim": {
        "api_url": "http://127.0.0.1:18080/v1",
        "available_models": [
          {
            "name": "gpt-5.5",
            "display_name": "GPT-5.5 via local shim",
            "max_tokens": 400000,
            "max_output_tokens": 32000,
            "max_completion_tokens": 32000,
            "capabilities": {
              "tools": true,
              "images": true,
              "parallel_tool_calls": true,
              "prompt_cache_key": true,
              "chat_completions": false
            }
          },
          {
            "name": "gpt-5.4",
            "display_name": "GPT-5.4 via local shim",
            "max_tokens": 400000,
            "max_output_tokens": 32000,
            "max_completion_tokens": 32000,
            "capabilities": {
              "tools": true,
              "images": true,
              "parallel_tool_calls": true,
              "prompt_cache_key": true,
              "chat_completions": false
            }
          },
          {
            "name": "gpt-5.3-codex",
            "display_name": "GPT-5.3 Codex via local shim",
            "max_tokens": 400000,
            "max_output_tokens": 32000,
            "max_completion_tokens": 32000,
            "capabilities": {
              "tools": true,
              "images": true,
              "parallel_tool_calls": true,
              "prompt_cache_key": true,
              "chat_completions": false
            }
          }
        ]
      }
    }
  },
  "agent": {
    "default_model": {
      "provider": "openai_shim",
      "model": "gpt-5.5",
      "enable_thinking": false
    }
  }
}
```

The important parts are:

- `api_url` ends with `/v1`
- `capabilities.chat_completions` is `false`, so Zed uses `/v1/responses`
- `capabilities.prompt_cache_key` can stay `true`; the shim preserves it
- `agent.default_model.provider` matches the provider name `openai_shim`

Do not put the real upstream URL or API key in Zed settings. Point Zed at the
local shim and keep upstream details in `.env`.

## Zed Agent Usage

With the release executable running and `agent.default_model` pointing to
`openai_shim`, Zed Agent can use the configured OpenAI-compatible models
through the local proxy.

![Zed Agent using GPT-5.5 via local shim](<docs/images/zed agent.png>)

The screenshot above shows Zed Agent selecting `GPT-5.5 via local shim` and
receiving a response through the configured provider.

![Shim forwarding a Zed Agent request](docs/images/log.png)

The shim log shows a forwarded `/v1/responses` request and an upstream `200`
response, which confirms the local proxy path is being used.

To verify manually, start `zed-openai-shim.exe`, open Zed Agent, select
`GPT-5.5 via local shim` or another configured `openai_shim` model, and send an
agent request. The shim should log a forwarded `/v1/responses` request.

When adding more screenshots, store them under `docs/images/`. Do not include
captures that expose API keys, private prompts, or repository secrets.

## Verified Behavior

Offline tests and the Rust proxy E2E are covered by `just check`. Observed
real-upstream behavior:

- raw `role:"system"` in `input`: upstream fails
- `system -> instructions`: upstream returns 200
- detailed sub2api issue notes:
  [docs/sub2api-system-role-issue.md](docs/sub2api-system-role-issue.md)

## Troubleshooting

If Zed Agent shows a connection error for
`http://127.0.0.1:18080/v1/responses`, the shim is usually not running or is
listening on a different port.

![Zed Agent error when the shim is not running](docs/images/error.png)

Fix it by starting the release executable from its release directory, then
retrying the Agent request:

```powershell
.\zed-openai-shim.exe
```

If you changed `OPENAI_SHIM_PORT`, make sure the Zed `api_url` uses the same
port and still ends with `/v1`.

## Logs

The shim logs to stdout/stderr. If file logs are needed, write them under:

```text
logs/
```

`logs/` is ignored by git.

## Upstream Notes

The upstream sub2api issue is
[Wei-Shaw/sub2api#2147](https://github.com/Wei-Shaw/sub2api/issues/2147), with
local analysis in
[docs/sub2api-system-role-issue.md](docs/sub2api-system-role-issue.md). This
shim remains useful while upstream support for Responses `input_text` system
content is pending.
