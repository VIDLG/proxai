# 配置说明

本文档说明 ProxAI 当前使用的运行时配置模型。

## 范围与当前运行时状态

ProxAI 当前稳定支持这些 no-conversion 转发路径：

- 入站：`openai_responses` -> 出站：`openai_responses`
- 入站：`openai_chat_completions` -> 出站：`openai_chat_completions`
- 入站：`anthropic_messages` -> 出站：`anthropic_messages`

跨协议转换保持显式，不会因为默认 provider 自动启用。

## 应用目录

运行时文件位于用户应用目录：

- Windows：`%USERPROFILE%\\.proxai\\`
- Linux/macOS：`~/.proxai/`

重要文件和目录：

- `config.toml`
- `config.example.toml`
- `logs/`
- `captures/`

## `[server]`

控制本地监听地址、端口和 HTTP 接入限制。

字段：

- `host`
- `port`
- `max_request_body_bytes`：本地代理接受的最大入站请求体大小
- `max_concurrent_requests`：本地代理同时处理的最大请求数

## `[mcp]`

配置本地 MCP 控制/API 监听器的 `host` 和 `port`。

字段：

- `host`
- `port`

当前 ProxAI 会在这个地址上启动一个 `/mcp` 的本地 streamable HTTP MCP endpoint。
使用默认配置时，就是：

- `http://127.0.0.1:18081/mcp`

## `[routing.default_provider_names]`

按**入站请求协议**声明默认 provider。

当前 key：

- `openai_responses`
- `openai_chat_completions`
- `anthropic_messages`

当没有任何 route 命中时，才使用这些默认 provider。

## `[[routing.routes]]`

route 是对入站请求的过滤规则。

字段：

- `name` 可选，但建议填写，便于 CLI 临时覆盖
- `request_protocol` 可选
- `match_kind` 可选
- `model_pattern`
- `provider`
- `upstream_model` 可选

### `name`

稳定的 route 标识，用于运行时 CLI override。它不参与匹配，但可以让你按
名称定位某条 route，而不是依赖 route 顺序。

单次运行覆盖示例：

```sh
proxai --route-override minimax_m3_chat.model_pattern=MiniMax-M3-preview \
  --route-override minimax_m3_chat.upstream_model=MiniMax-M3
```

支持覆盖的字段包括 `request_protocol`、`match_kind`、`model_pattern`、
`provider` 和 `upstream_model`。对可选字段 `request_protocol` 或
`upstream_model` 传空值可以清空配置。

### `request_protocol`

这是 route 的过滤条件，不是客户端手工传入的字段。

运行时会先根据实际请求 path（`/v1/responses`、`/v1/chat/completions` 或
`/v1/messages`）自动识别入站协议。route 中的 `request_protocol` 只是对这个
已识别协议的可选保护条件。

如果省略 `request_protocol`，这条 route 可以匹配任意入站协议。所选 provider
的 `protocol` 仍然决定转发给上游时使用的 wire format，所以省略
`request_protocol` 也可以有意把 OpenAI Chat Completions 或 OpenAI Responses
请求路由到 Anthropic Messages provider。

如果显式设置了 `request_protocol`，并且模型模式命中但实际入站协议不同，
ProxAI 会返回配置错误，而不是静默落到默认 provider。只有当同一个
`model_pattern` 需要按不同请求端点走不同 route 时，才建议显式设置
`request_protocol`。

### `match_kind`

支持：

- `exact`
- `glob`
- `regex`
- `auto`

省略时默认是 `auto`。

### `model_pattern`

逻辑模型匹配字段。

例如：

- `gpt-5.4`
- `gpt-*`
- `^claude-(?<tier>.+)$`

### `provider`

命中该 route 时要选择的 provider 名称。

### `upstream_model`

可选的上游模型映射字段。

语义：

- 省略：原始请求模型名原样透传
- `exact` / `glob`：当固定上游模型名
- `regex`：当 regex replacement template，支持 `$1` 或 `$name`

## `[providers.<name>]`

每个 provider 描述 ProxAI 如何连接一个具体上游。
下面这些字段都必须显式填写。

字段：

- `protocol`
- `base_url`
- `api_key`
- `compatibility` 可选
- `read_idle_timeout_secs`

当前协议值：

- `openai_responses`
- `openai_chat_completions`
- `anthropic_messages`

### Provider key 覆盖行为

`api_key` 是必填项。
对于 OpenAI provider（`openai_responses` 和 `openai_chat_completions`），ProxAI 会把这个 key 作为 `Authorization: Bearer <key>` 发给上游，并忽略客户端原本传来的 `Authorization` header。对于 Anthropic Messages provider，ProxAI 会发送 `x-api-key`。

因此，如果 Zed UI 要求填写 key，通常可以放一个 dummy key，而把真实上游 key 保留在 ProxAI 的配置里。

### `compatibility`

支持：

- `strict`
- `anthropic_compatible`

对于 `anthropic_messages` provider，`anthropic_compatible` 可能会在 ProxAI
记录日志、转换协议或返回成功响应前补齐兼容性缺口。目前它会把缺失的 SDK
required-nullable 响应字段补成显式 `null`，把裸 `message_start` 事件
规范成官方的嵌套 `message` 结构，修补已测到的 MiniMax 场景：流式
thinking `content_block_start` 省略 `signature`，并修补已测到的 GLM 5.1
场景：`server_tool_use` 只包含一个 counter。

不要为其他 provider-specific 缺失字段直接填默认业务值，例如缺失的 tool
caller，除非它们有明确测量到的上游 case 和聚焦 fixture 支撑。

官方 Anthropic API 或已经严格输出官方 Messages schema 的上游，建议使用
`strict`。如果省略，默认是 `anthropic_compatible`，优先保证本地兼容性。

## `[tool_calls]`

`timeout_secs` 是 streamed tool-call arguments 的语义超时。

它始终启用，且必须大于 0。

## `[capture]`

- `inbound_request_enabled = true` 时，会把 proxai 收到的客户端请求写到 app-dir 下预定义的 `captures/` 目录。
- `inbound_request_enabled = false` 时，不写 inbound request capture。
- `provider_request_enabled = true` 时，会把 proxai 适配后实际发往上游的请求写出来。
- `provider_request_enabled = false` 时，不写 provider request capture。
- `upstream_response_enabled = true` 时，会把 upstream response headers 和原始 upstream response bytes 写到 capture 目录。
- `upstream_response_enabled = false` 时，不写 upstream response capture。
- `outbound_response_enabled = true` 时，会把 proxai 最终返回给客户端的响应写出来。
- `outbound_response_enabled = false` 时，不写 outbound response capture。

capture 路径不允许在配置里改。ProxAI 启动时总会准备 app-dir 下的
`captures/` 目录；这些 phase 开关只控制请求是否往里面写 artifact。

这里是运行时默认值；如果要持久修改本地默认值，可以用：

- `proxai capture status`
- `proxai capture enable [inbound-request|provider-request|upstream-response|outbound-response]`
- `proxai capture disable [inbound-request|provider-request|upstream-response|outbound-response]`

如果只是临时调试，也可以通过 CLI 覆盖单次运行中的任意 capture phase。

## `[logging]` 与 `[logging.duration_thresholds]`

- `output_format = "human"`：适合人工排查
- `output_format = "json"`：适合机器消费
- `use_color = true`：启用 human 日志颜色
- `use_color = false`：禁用 human 日志颜色
- `warn_ms` / `error_ms`：控制 human 日志颜色阈值

## `[error_responses]`

- `text`：适合 Zed 阅读
- `json`：适合非 Zed 客户端

对于上游非 2xx 响应，ProxAI 会归一化响应体，并保留 `Retry-After`、
上游 request id、rate-limit headers 等有助于排障的响应头。

## Timeout 语义

### `read_idle_timeout_secs`

这不是整条请求的总时长上限。
它表示：在读取上游响应的过程中，如果连续这么久没有任何新 bytes 到来，就超时。

### `[tool_calls].timeout_secs`

这是另一个语义层超时，专门防止上游开始流式发送 tool-call arguments 后却一直不收尾，导致客户端无限等待。
