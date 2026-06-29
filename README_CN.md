# ProxAI

📚 **文档站**: [vidlg-proxai.netlify.app](https://vidlg-proxai.netlify.app)（基于 Astro / Starlight，源在 `site/`）

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

当前稳定可用的转发与转换路径：

- 入站：`openai_responses` → 出站：`openai_responses`
- 入站：`openai_chat_completions` → 出站：`openai_chat_completions`
- 入站：`anthropic_messages` → 出站：`anthropic_messages`
- 入站：`openai_responses` → 出站：`openai_chat_completions`
- 入站：`openai_responses` → 出站：`anthropic_messages`
- 入站：`openai_chat_completions` → 出站：`anthropic_messages`
- 入站：`anthropic_messages` → 出站：`openai_responses`

其他跨协议转换路径仍保持显式未支持，直到逐个实现。完整矩阵见
[协议参考](https://vidlg-proxai.netlify.app/zh/reference/protocols)。

## 快速开始

1. 下载 Windows release 可执行文件，或从源码构建。
2. 先运行一次 ProxAI，让应用目录和 `config.example.toml` 自动生成。
3. 编辑 `config.toml`（Windows 在 `%USERPROFILE%\.proxai\`，Linux/macOS 在 `~/.proxai/`），把 provider 的 `base_url` 和 `api_key` 配好。
4. 把 OpenAI 兼容客户端指向 `http://127.0.0.1:18080/v1`。

完整步骤见 [快速开始](https://vidlg-proxai.netlify.app/zh/using/quick-start)。

## 默认端点

| 端点 | 默认 URL |
|---|---|
| Proxy | `http://127.0.0.1:18080` |
| MCP | `http://127.0.0.1:18081/mcp` |

其他默认值与限制见 [默认值与限制](https://vidlg-proxai.netlify.app/zh/reference/defaults-and-limits)。

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

完整参考（含 `capture` 子命令）见 [CLI 参考](https://vidlg-proxai.netlify.app/zh/reference/cli)。

## 文档

完整文档位于 `site/src/content/docs/`，并发布到
[vidlg-proxai.netlify.app](https://vidlg-proxai.netlify.app)。主要章节：

- [使用 ProxAI](https://vidlg-proxai.netlify.app/zh/using) —— 面向用户的任务指南
- [配置说明](https://vidlg-proxai.netlify.app/zh/using/configuration) —— server、routing、providers、capture、logging、errors
- [路由与 Provider](https://vidlg-proxai.netlify.app/zh/using/routing-and-providers) —— provider 如何被选中
- [观测与诊断](https://vidlg-proxai.netlify.app/zh/using/observability) —— capture、日志、隐私边界
- [常见排障](https://vidlg-proxai.netlify.app/zh/using/troubleshooting) —— 常见症状与下一步检查
- [协议总览](https://vidlg-proxai.netlify.app/zh/protocol) —— phase 轴、protocol 轴、转换矩阵
- [流式行为](https://vidlg-proxai.netlify.app/zh/protocol/streaming-behavior) —— terminal event、tool-call 超时
- [架构](https://vidlg-proxai.netlify.app/zh/developer/architecture) —— 请求生命周期、模块边界
- [行为契约](https://vidlg-proxai.netlify.app/zh/reference/behavior-contracts) —— ProxAI 跨版本承诺的稳定行为

参考页：

- [配置参考](https://vidlg-proxai.netlify.app/zh/reference/configuration) —— 完整 `config.example.toml`
- [CLI](https://vidlg-proxai.netlify.app/zh/reference/cli) —— 运行 flag 与 capture 子命令
- [默认值与限制](https://vidlg-proxai.netlify.app/zh/reference/defaults-and-limits)
- [协议](https://vidlg-proxai.netlify.app/zh/reference/protocols) —— 取值、path、转换对
- [路由匹配](https://vidlg-proxai.netlify.app/zh/reference/route-matching) —— route 结果、协议 guard 与 fallback 行为
- [Capture Phases](https://vidlg-proxai.netlify.app/zh/reference/capture-phases) —— capture 边界与隐私风险
- [环境与文件](https://vidlg-proxai.netlify.app/zh/reference/environment-and-files) —— app 目录和本地产物
- [错误响应](https://vidlg-proxai.netlify.app/zh/reference/error-responses) —— payload、type 枚举、HTTP status
- [术语表](https://vidlg-proxai.netlify.app/zh/reference/glossary) —— 共享术语

## 开发

常用命令：

- `pixi install`
- `just run` —— 本地运行 ProxAI
- `just check` —— 完整本地校验
- `just test-e2e` —— 端到端测试
- `just build` —— release 构建
- `cargo run -- check-update` —— 检查更新

与官方 SDK 的协议类型覆盖率对比：

- `just compare-anthropic-protocol` —— Anthropic Messages 协议类型 vs 官方 TS SDK
- `just compare-openai-protocol` —— OpenAI 协议类型 vs `async-openai` v0.40.2

这些用于对比的 SDK checkout 作为 git submodule 放在 `contrib/`：

- `contrib/anthropic-sdk-typescript`
- `contrib/async-openai`

这些脚本强制执行的对齐规则见 [协议转换](https://vidlg-proxai.netlify.app/zh/developer/protocol-conversion)。

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
