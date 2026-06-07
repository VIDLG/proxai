# ProxAI

[English README](README.md)

ProxAI 是一个运行在 AI 客户端和模型上游之间的本地轻量兼容代理。
它接收本地 OpenAI Compatible 请求，修复会导致某些上游失败的特定
OpenAI Responses API system-message 形状问题，然后尽量透明地转发到
配置好的 provider。

当前稳定运行路径支持 OpenAI Responses、OpenAI Chat Completions 和
Anthropic Messages 的 no-conversion 转发，也支持若干显式跨协议转换路径。
配置模型已经按协议感知整理好，后续可以显式扩展到更多转换路径与路由，
但不会因此演变成泛化的 AI 网关。

## 当前状态

当前稳定可用的转发与转换路径是：

- 入站：`openai_responses` -> 出站：`openai_responses`
- 入站：`openai_chat_completions` -> 出站：`openai_chat_completions`
- 入站：`anthropic_messages` -> 出站：`anthropic_messages`
- 入站：`openai_responses` -> 出站：`openai_chat_completions`
- 入站：`openai_responses` -> 出站：`anthropic_messages`
- 入站：`openai_chat_completions` -> 出站：`anthropic_messages`
- 入站：`anthropic_messages` -> 出站：`openai_responses`

其他跨协议转换路径仍保持显式未支持，直到逐个实现。

## ProxAI 现在会做什么

对 JSON `/v1/responses` 请求，ProxAI 目前会处理一个很具体的兼容问题：

- 查找顶层 `input` 里 `role:"system"` 的 item
- 提取其中 `input_text` / `text` 内容
- prepend 到顶层 `instructions`
- 从 `input` 里删除原 system item
- 其他字段尽量保持不变

这样可以让客户端继续使用那些不接受 Responses 风格 `input` 内 system
message 的上游。

对 `/v1/chat/completions` 请求，ProxAI 会校验 Chat Completions 请求形状，
应用 provider 路由 / 模型改写；如果路由选择 OpenAI Chat Completions
provider，则原协议转发；如果显式路由到 `anthropic_messages` provider，
则转换为 Anthropic Messages。

对 `/v1/messages` 请求，ProxAI 会校验 Anthropic Messages 请求形状，应用
provider 路由 / 模型改写；如果路由选择 Anthropic Messages provider，则原协议
转发；如果显式路由到 `openai_responses` provider，则转换为 OpenAI Responses。

## 安装与应用目录

下载 Windows release 可执行文件，先运行一次，然后去用户应用目录编辑
自动生成的配置文件。

运行时文件位于：

- Windows：`%USERPROFILE%\\.proxai\\config.toml`
- Windows：`%USERPROFILE%\\.proxai\\config.example.toml`
- Linux/macOS：`~/.proxai/config.toml`
- Linux/macOS：`~/.proxai/config.example.toml`

同目录下还会有：

- `logs/`
- `captures/`

首次真正使用前，需要先在 `config.toml` 里把相关 provider 的 `base_url`
和 `api_key` 配好。

## 运行

配置好之后：

- 可执行文件名：`proxai.exe`
- 默认代理监听地址：`http://127.0.0.1:18080`
- 默认 MCP endpoint：`http://127.0.0.1:18081/mcp`

CLI 覆盖项保持精简：

- `--config`
- `--upstream`
- `--api-key`
- `--port`
- `--log-level`
- `--log-format`
- `--route-override ROUTE.FIELD=VALUE`

其中 `--upstream` 和 `--api-key` 会临时覆盖本次运行中
`routing.default_provider_names.openai_responses` 所选 provider 的上游地址和 key。
`--route-override` 会按名称临时覆盖某条 `[[routing.routes]]` 的字段，例如：

```sh
proxai --route-override minimax_m3_chat.model_pattern=MiniMax-M3-preview
```

## 配置概览

运行时配置在 `config.toml`，跟踪示例在 `config.example.toml`。

完整字段说明请看：

- [docs/configuration_cn.md](docs/configuration_cn.md)

简要来说，配置主要围绕这些部分组织：

- `[server]`（监听地址，以及请求体大小和并发限制）
- `[mcp]`
- `[routing.default_provider_names]`
- `[[routing.routes]]`
- `[providers.<name>]`
- `[tool_calls]`
- `[capture]`（`inbound_request_enabled` / `provider_request_enabled` / `upstream_response_enabled` / `outbound_response_enabled`）
- `[logging]`
- `[error_responses]`

当前稳定运行路径包括 OpenAI Responses、OpenAI Chat Completions 和
Anthropic Messages 的 no-conversion 转发，以及
`openai_responses -> openai_chat_completions`、
`openai_responses -> anthropic_messages`、
`openai_chat_completions -> anthropic_messages`、
`anthropic_messages -> openai_responses` 这些显式转换。provider/routing
模型已经提前按显式多协议扩展整理好了。

route 的 `request_protocol` 是可选项。省略时，这条 route 可以匹配由请求
path 识别出的任意入站协议；显式设置时，如果模型命中但实际入站协议不同，
会作为配置错误返回。

Anthropic Messages provider 如果连接官方 Anthropic API，建议使用
`compatibility = "strict"`；如果连接会省略部分官方响应字段的兼容上游，
使用 `compatibility = "anthropic_compatible"`。

对于上游非 2xx 响应，ProxAI 会归一化响应体，并保留 `Retry-After`、
上游 request id、rate-limit headers 等有助于排障的响应头。

`[mcp]` 现在会配置一个本地 MCP 监听器。默认情况下，ProxAI 会启动一个 streamable HTTP MCP endpoint：`http://127.0.0.1:18081/mcp`。

## 当前客户端配置建议

对于 OpenAI Compatible 客户端，可以配置一个 provider 指向：

- `http://127.0.0.1:18080/v1`

客户端里建议只暴露逻辑模型名，比如：

- `gpt-5.4`
- `gpt-5.5`
- 未来可扩展到 `claude-sonnet`

实际走哪个 provider、上游真实模型名是什么，都交给 ProxAI 在
`~/.proxai/config.toml` 里路由。

## 开发

常用命令：

- Rust toolchain：stable，需支持 Rust 2024 edition
- `pixi install`
- `just run`
- `just check`
- `just test-e2e`
- `just build`
- `cargo run -- check-update`

与官方 SDK 的协议类型覆盖率对比：

- `just compare-anthropic-protocol` — Anthropic Messages 协议类型 vs 官方 TS SDK
- `just compare-openai-protocol` — OpenAI 协议类型 vs `async-openai` v0.38

这些用于对比的 SDK checkout 作为 git submodule 放在 `contrib/`：

- `contrib/anthropic-sdk-typescript`
- `contrib/async-openai`

支持 `-d`（详细，默认）、`-q`（简洁）、`-v`（冗长+分类）三个输出级别。

常用 capture 控制命令：

- `cargo run -- capture status`
- `cargo run -- capture enable`
- `cargo run -- capture disable`
- `cargo run -- capture enable inbound-request`
- `cargo run -- capture enable provider-request`
- `cargo run -- capture enable upstream-response`
- `cargo run -- capture enable outbound-response`

常用临时调试覆盖项：

- `cargo run -- --capture-inbound-request`
- `cargo run -- --capture-provider-request`
- `cargo run -- --capture-upstream-response`
- `cargo run -- --capture-outbound-response`

本地 release 可执行文件：

- `target\\release\\proxai.exe`

## 协议类型对齐策略

ProxAI 的协议类型遵循严格的**名称一致性**规则：

1. **不使用类型别名** — 每个 SDK 类型名在 proxai 中有且只有一个
   `pub struct` 或 `pub enum`，绝不使用 `pub type X = Y`。
2. **不折叠类型** — 当 SDK 区分 `*Block` 和 `*BlockParam`（或类似的
   请求/响应类型对）时，proxai 为每个类型保留独立的结构体。
3. **不改名** — proxai 使用 SDK 原生的类型名，即使 SDK 的大小写
   不一致（使用 `Base64PdfSource` 而非 `Base64PDFSource`）。
4. **字符串联合作为枚举** — SDK 中的固定字符串联合类型
   （`Array<'direct' | 'code_execution_20250825'>`）建模为
   `Vec<EnumType>` 而非 `Vec<String>`。

这些规则由 `tools/compare_anthropic_protocol.py` 和
`tools/compare_openai_protocol.py` 强制执行。它们使用 tree-sitter
AST 解析，逐字段对比 proxai 类型与官方 SDK，报告缺失类型、缺失字段、
字段顺序不匹配、serde wire 语义以及废弃字段自动排除，支持三个详细级别。
当 SDK 的 required-nullable 字段（`field: T | null`）在 Rust 中表示为
`Option<T>` 时，该字段必须标注
`/// @sdk(required_nullable_accepts_missing)`，表示 proxai 有意接受字段缺失
作为兼容宽松。完整的转换和对齐规则见 `docs/protocol-conversion_cn.md`。

## Release 产物

GitHub Release 产物命名类似：

- `proxai-vX.Y.Z-windows-x86_64.exe`

## 关于未来协议

当前仓库的跨协议 translation 和 route-level protocol filter 都保持显式。
新增协议对时，应逐个补齐 runtime 路由、请求 / 响应转换和对应测试，避免
ProxAI 在无意中变成通用 AI 平台。
