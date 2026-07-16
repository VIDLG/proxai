# ProxAI

📚 **文档站**: [vidlg.github.io/proxai](https://vidlg.github.io/proxai)（基于 Astro / Starlight，源在 `site/`）

[English README](README.md)

ProxAI 是一个运行在 AI 客户端和模型上游之间的本地轻量兼容代理。
它接收本地 OpenAI Compatible 请求，修复会导致某些上游失败的、Zed 特有的
OpenAI Responses API system-message 与紧凑历史消息形状问题，然后尽量透明地
转发到配置好的 provider。

当前稳定运行路径支持 OpenAI Responses、OpenAI Chat Completions 和
Anthropic Messages 的 no-conversion 转发，也支持若干显式跨协议转换路径。
配置模型已经按协议感知整理好，后续可以显式扩展到更多转换路径与路由，
但不会因此演变成泛化的 AI 网关。

## 当前状态

当前稳定可用的转发与转换路径：

- 入站：`openai_responses` → 出站：`openai_responses`
- 入站：`openai_chat_completions` → 出站：`openai_chat_completions`
- 入站：`anthropic_messages` → 出站：`anthropic_messages`
- 入站：`openai_responses` → 出站：`openai_chat_completions`
- 入站：`openai_responses` → 出站：`anthropic_messages`
- 入站：`openai_chat_completions` → 出站：`anthropic_messages`
- 入站：`openai_chat_completions` → 出站：`openai_responses`
- 入站：`anthropic_messages` → 出站：`openai_responses`
- 入站：`anthropic_messages` → 出站：`openai_chat_completions`

完整矩阵和各 protocol pair 的信息损失说明见
[协议参考](https://vidlg.github.io/proxai/zh/reference/protocols)。

对于 Zed 等 Chat-compatible 客户端，普通 reasoning 文本会通过
`reasoning_content` 扩展保留在 assistant 历史消息、非流式 message 和流式
delta 中。Zed 的流式响应还接受 `reasoning`，但 assistant 历史回放使用
`reasoning_content`。这些兼容字段只在 translation 边界提取和注入，官方
OpenAI Chat wire types 仍严格对齐 OpenAPI schema。Zed Responses 请求历史则在
严格解析前单独归一化：ProxAI 会补全紧凑 assistant output envelope、缺少
schema-required `id` 的 reasoning summary item，以及 message image 省略的 `detail`
默认值，同时保持官方 Responses wire types 不变。在 provider compatibility 模式下，
ProxAI 还会修复已测量到的上游缺失字段，例如 MiniMax Chat 流式 chunk
缺少 required-nullable `choices[].finish_reason`。ProxAI 也会接受 Bedrock Mantle
坐标可缺省的 `response.reasoning.delta/done` 事件，而不伪造 Responses item
身份。Redacted 或 encrypted reasoning 不会伪装成普通可见正文。

Anthropic thinking 的跨轮续接使用带版本的、由客户端携带的 continuation envelope：
Responses 放在 `reasoning.encrypted_content`，Chat-compatible 历史则附在
`reasoning_content` 后。ProxAI 会在向 Anthropic 转发下一次请求前剥离 envelope
并恢复 provider-specific thinking block；代理自身不保存这类续接状态。

请求和流式 translation 失败都会自动在 `diagnostics/` 下生成有数量上限的本地
诊断 bundle，独立于 capture。流式 bundle 会仅在本地保留触发失败的原始 SSE frame；
普通日志只输出安全上下文和 `diag=...`。

## 快速开始

1. 下载 Windows release 可执行文件，或从源码构建。
2. 先运行一次 ProxAI，让应用目录和 `config.example.toml` 自动生成。
3. 编辑 `config.toml`（Windows 在 `%USERPROFILE%\.proxai\`，Linux/macOS 在 `~/.proxai/`），把 provider 的 `base_url` 和 `api_key` 配好。
4. 把 OpenAI 兼容客户端指向 `http://127.0.0.1:18080/v1`。

完整步骤见 [快速开始](https://vidlg.github.io/proxai/zh/using/quick-start)。

## 默认端点

| 端点 | 默认 URL |
|---|---|
| Proxy | `http://127.0.0.1:18080` |
| MCP | `http://127.0.0.1:18081/mcp` |

其他默认值与限制见 [默认值与限制](https://vidlg.github.io/proxai/zh/reference/defaults-and-limits)。

## CLI

CLI flag 刻意保持精简，仅用于临时覆盖：

```sh
proxai --config <path> \
       --upstream <url> \
       --api-key <key> \
       --port <port> \
       --log-level <level> \
       --log-format <human|json> \
       --route-override ROUTE.FIELD=VALUE
```

完整参考（含 `capture` 子命令）见 [CLI 参考](https://vidlg.github.io/proxai/zh/reference/cli)。

## 文档

完整文档位于 `site/src/content/docs/`，并发布到
[vidlg.github.io/proxai](https://vidlg.github.io/proxai)。主要章节：

- [使用 ProxAI](https://vidlg.github.io/proxai/zh/using) —— 面向用户的任务指南
- [配置说明](https://vidlg.github.io/proxai/zh/using/configuration) —— server、routing、providers、capture、logging、errors
- [路由与 Provider](https://vidlg.github.io/proxai/zh/using/routing-and-providers) —— provider 如何被选中
- [观测与诊断](https://vidlg.github.io/proxai/zh/using/observability) —— 紧凑日志、自动失败诊断、capture 与隐私边界
- [常见排障](https://vidlg.github.io/proxai/zh/using/troubleshooting) —— 常见症状与下一步检查
- [协议总览](https://vidlg.github.io/proxai/zh/protocol) —— phase 轴、protocol 轴、转换矩阵
- [流式行为](https://vidlg.github.io/proxai/zh/protocol/streaming-behavior) —— terminal event、tool-call 超时
- [架构](https://vidlg.github.io/proxai/zh/developer/architecture) —— 请求生命周期、模块边界
- [行为契约](https://vidlg.github.io/proxai/zh/reference/behavior-contracts) —— ProxAI 跨版本承诺的稳定行为

参考页：

- [配置参考](https://vidlg.github.io/proxai/zh/reference/configuration) —— 完整 `config.example.toml`
- [CLI](https://vidlg.github.io/proxai/zh/reference/cli) —— 运行 flag 与 capture 子命令
- [默认值与限制](https://vidlg.github.io/proxai/zh/reference/defaults-and-limits)
- [协议](https://vidlg.github.io/proxai/zh/reference/protocols) —— 取值、path、转换对
- [路由匹配](https://vidlg.github.io/proxai/zh/reference/route-matching) —— route 结果、协议 guard 与 fallback 行为
- [Capture Phases](https://vidlg.github.io/proxai/zh/reference/capture-phases) —— capture 边界与隐私风险
- [环境与文件](https://vidlg.github.io/proxai/zh/reference/environment-and-files) —— app 目录和本地产物
- [错误响应](https://vidlg.github.io/proxai/zh/reference/error-responses) —— payload、type 枚举、HTTP status
- [术语表](https://vidlg.github.io/proxai/zh/reference/glossary) —— 共享术语

## 开发

常用命令：

- `pixi install`
- `just run` —— 本地运行 ProxAI
- `just check` —— 完整本地校验，包含 OpenAI 协议漂移检查
- `just test-e2e` —— 端到端测试
- `just build` —— release 构建
- `cargo run -- check-update` —— 检查更新

与官方协议参考的覆盖率对比：

- `just protocol-compare` —— `just check` 使用的 OpenAI schema required-field 漂移检查
- `just compare-openai-protocol` —— 基于官方 OpenAPI schema 的详细 required-field 检查；传入 `--structural` 可执行手动 schema-first 结构审计，检查未映射 wire 类型与未建模属性（不纳入 CI）
- `just compare-anthropic-protocol` —— Anthropic Messages 类型 vs 官方 TS SDK

这些协议参考 checkout 作为 git submodule 放在 `contrib/`：

- `contrib/openai-openapi`
- `contrib/anthropic-sdk-typescript`

这些脚本强制执行的对齐规则见 [协议转换](https://vidlg.github.io/proxai/zh/developer/protocol-conversion)。

## 文档站

文档站基于 Astro + Starlight。从仓库根目录：

```sh
just site install   # 安装依赖（pnpm via pixi）
just site dev       # 本地 dev server，http://localhost:4321
just site build     # 生产构建到 site/dist
just site check     # 构建 + 文档 i18n/结构校验
```

详见 [`site/README.md`](site/README.md)。

## Release 产物

GitHub Release 产物命名类似：

- `proxai-vX.Y.Z-windows-x86_64.exe`

## 关于未来协议

当前仓库的跨协议 translation 和 route-level protocol filter 都保持显式。
新增协议对时，应逐个补齐 runtime 路由、请求 / 响应转换和对应测试，避免
ProxAI 在无意中变成通用 AI 平台。
