# 协议转换与 Wire Model 对齐

English version: [`protocol-conversion.md`](protocol-conversion.md)

ProxAI 保持协议转换显式、按协议对组织。本文记录维护 wire model、转换代码和 SDK 对齐检查时应遵循的规则。

## 边界

- `src/protocol/` 负责协议专属的 Rust wire model。
- `src/ingress/` 负责翻译前的入站解析与归一化。
- `src/translation/` 负责在入站 `request_protocol` 和出站 provider `protocol` 之间做纯跨协议转换。
- `src/provider/request.rs` 负责 provider 请求准备，包括模型改写、projection/summary 提取和 JSON body 序列化。
- `src/provider/transport.rs` 负责出站 HTTP 传输、认证头、上游 URL 构造和发送。
- `src/http_support/` 负责 HTTP carrier 辅助能力，例如 `ByteStream`、content-type/header 辅助和响应重建。

不要把通用跨协议转换隐藏在 provider 子树里。Provider 代码可以处理 provider 本地兼容性怪癖，但协议到协议的 shape 改写应该放在 `src/translation/`。

Translation API 在 carrier 边界应保持纯粹：

- 请求转换：`(request_protocol, provider_protocol, normalized_payload) -> payload`
- 非流式响应转换：`(request_protocol, provider_protocol, payload) -> payload`
- 流式响应转换：`(request_protocol, provider_protocol, ByteStream) -> ByteStream`

不要把 HTTP `Response`、`Body`、provider request struct 或路由/模型改写细节传进 `src/translation/`。

## 命名

使用协议名描述 wire 行为：

- `openai_responses`
- `openai_chat_completions`
- `anthropic_messages`

转换模块使用成对命名，例如：

- `openai_responses -> anthropic_messages`
- `anthropic_messages -> openai_responses`

Provider 名称是用户标签，不应被当成语义协议标识。

## 路由与转换

路由可以指定 `request_protocol`。如果省略，该路由可以匹配任意从实际请求路径检测出的入站协议。Provider `protocol` 控制出站 wire 格式，因此路由协议过滤和协议转换是两个独立决策。

只有当同一个 model pattern 需要按不同请求 endpoint 分流时，才设置 `request_protocol`。如果 model pattern 匹配，但显式 `request_protocol` 与入站请求协议不同，ProxAI 会报告配置错误，而不是静默落到默认 provider。

## Refusal 与普通内容语义

`refusal` 表示模型生成的拒答内容，不是普通 assistant 文本上的附加标注。跨协议转换时应让它和普通文本保持区分。

三个支持的协议用不同方式表达这种区分：

| 协议 | 普通 assistant 文本 | Refusal | 普通文本和 refusal 能否在同一条 assistant message 中并存？ |
| --- | --- | --- | --- |
| `openai_responses` | `output[].content[]` 中 `type: "output_text"` 的 part | `output[].content[]` 中 `type: "refusal"` 的 part | 结构上可以作为不同 content part 并存，但语义上不常见；当目标协议能表达 part 时应保留顺序。 |
| `openai_chat_completions` | `choices[].message.content` 或流式 `delta.content` | `choices[].message.refusal` 或流式 `delta.refusal` | wire 字段都是 nullable/optional，但 refusal 不应在 `content` 中重复同一段文字。Assistant request content parts 也说明：要么是一个或多个 `text` part，要么是正好一个 `refusal` part。 |
| `anthropic_messages` | `content[]` 中的 `text` block | `stop_reason: "refusal"` 加可选 `stop_details.explanation`；可见拒答文字也可能以 `text` block 出现 | 没有单独的 refusal content block。被拒答的 message 仍可能包含可见 text block，因此 translator 必须结合上下文判断这些 text block 是拒答文字还是普通内容。 |

### OpenAI Responses

Responses 将 message content 保持为 typed parts，因此普通文本和 refusal 是同一个 `content[]` 数组中的不同值：

```json
{
  "type": "message",
  "role": "assistant",
  "status": "completed",
  "content": [
    {
      "type": "output_text",
      "text": "I can help with safe alternatives.",
      "annotations": []
    },
    {
      "type": "refusal",
      "refusal": "I can't provide instructions for that request."
    }
  ]
}
```

如果目标协议能保留 typed content parts，就保留二者区别。如果目标协议是 Chat Completions，除非目标侧没有 refusal 字段，否则不要把 refusal text 合并进普通 `message.content`。

### OpenAI Chat Completions

Chat response message 将普通内容和 refusal 暴露为同级字段：

```json
{
  "role": "assistant",
  "content": null,
  "refusal": "I can't provide instructions for that request."
}
```

JSON shape 在顶层没有让 `content` 和 `refusal` 强制互斥，但二者语义不同。不要在两个字段中输出同一段拒答文本：

```json
{
  "role": "assistant",
  "content": "I can't provide instructions for that request.",
  "refusal": "I can't provide instructions for that request."
}
```

应把这种重复 shape 视为需要避免生成的兼容性产物，而不是理想输出。

Assistant request content parts 更明确地表达了这种区分：数组可以包含一个或多个 `text` part，或者正好一个 `refusal` part。这也强化了语义规则：refusal 是一种替代的 content kind，而不是普通文本上的装饰。

流式响应中也有相同区分：

```text
data: {"choices":[{"index":0,"delta":{"refusal":"I can't help with that."},"finish_reason":null}]}
data: {"choices":[{"index":0,"delta":{},"finish_reason":"stop"}]}
data: [DONE]
```

如果普通 `delta.content` 已经被转发，而后续上游事件才说明这一轮是 refusal，那么 stream 无法撤回已发内容。这种情况下不要再发送重复的 refusal 文本；只有当 refusal 能在普通内容发出前表达时，才使用 `delta.refusal`。

### Anthropic Messages

Anthropic 没有专门的 refusal content block。拒答由 message 级字段识别：

```json
{
  "id": "msg_01",
  "type": "message",
  "role": "assistant",
  "content": [
    {
      "type": "text",
      "text": "I can't provide instructions for that request."
    }
  ],
  "stop_reason": "refusal",
  "stop_details": {
    "category": "safety",
    "explanation": "The request asks for unsafe instructions."
  }
}
```

Anthropic -> Chat Completions 非流式转换规则：

- 当 `stop_reason == "refusal"` 且存在可见 text block 时，把展平后的可见文本放入 `message.refusal`，并让 `message.content` 缺省/null；
- 当 `stop_reason == "refusal"` 且没有可见 text 时，使用 `stop_details.explanation` 作为 fallback `message.refusal`；
- 不映射 `stop_details.category`，因为 Chat Completions 没有等价字段；
- 将 choice `finish_reason` 映射为 `stop`，因为 refusal 是一个终止的 assistant turn，不是工具调用。

目标 Chat response 示例：

```json
{
  "choices": [
    {
      "index": 0,
      "message": {
        "role": "assistant",
        "refusal": "I can't provide instructions for that request."
      },
      "finish_reason": "stop"
    }
  ]
}
```

Anthropic -> Chat Completions 流式转换中，`message_delta.stop_reason` 和 `stop_details` 会在 content block delta 之后到达。因此 proxai 使用 best-effort 规则：

- 如果还没有发出 text delta，将 `stop_details.explanation` 转成 `delta.refusal`；
- 如果 text 已经作为 `delta.content` 发出，则不要再发重复 refusal 文本；
- 最终 choice `finish_reason` 仍映射为 `stop`。

这不如缓冲整条 stream 后再严格重建 refusal 语义精确，但可以保留低延迟流式体验，并避免撤回已经转发的内容。

## SDK 对齐

Anthropic Messages wire model 会与 vendored 官方 TypeScript SDK（位于 `contrib/anthropic-sdk-typescript`）比较：

```sh
just compare-anthropic-protocol
```

比较内容包括类型覆盖、字段覆盖和顺序、serde discriminator 处理、枚举字面量、untagged union、结构化 SDK marker，以及选定的 serde 字段语义。

## Required-nullable 字段

TypeScript 区分以下两种 shape：

```ts
field?: T          // optional：字段可以不存在
field: T | null    // required nullable：字段应该存在，但可以为 null
```

Rust `Option<T>` 在反序列化时同时接受缺失和 `null`，因此它既不比任一 shape 更严格。对 SDK optional 字段来说足够精确；但对 SDK required-nullable 字段来说，它是有意更宽松的表示。

当 SDK required-nullable 字段用 `Option<T>` 表示时，应直接在 Rust 字段上标记：

```rust
pub struct Usage {
    pub output_tokens: u32,
    /// @sdk(required_nullable_accepts_missing)
    pub server_tool_use: Option<ServerToolUsage>,
}
```

这个 marker 表示：

- SDK shape：`field: T | null`
- Rust shape：`Option<T>`
- 有意差异：ProxAI 也接受字段缺失，作为兼容性容忍

当 SDK 字段是 optional（`field?: T` 或 `field?: T | null`）时，不要使用这个 marker。缺失本来就是官方 shape 的一部分。

不要用这个 marker 为 SDK required non-null 字段（`field: T`）使用 `Option<T>` 找理由。这类字段应该在 Rust 中保持非 optional，除非有另一个单独记录的协议决策。

Compare 脚本会在 `Required-nullable fields accepting missing` 小节中紧凑打印被标记字段。未标记的 required-nullable `Option<T>` 字段会导致比较失败。

## 兼容性归一化

Provider 兼容性归一化只能把保守或已测量到的上游偏差修复为最接近的官方协议 shape。当前保守修复包括：JSON 对象中缺失的 SDK required-nullable 响应字段（`missing -> null`），以及裸 `message_start` 事件归一化为官方嵌套 `message` shape。当前已测量的 provider 修复包括：

- MiniMax-compatible streams 可能在 thinking `content_block_start` 上省略 `signature`，因此 ProxAI 只针对这个窄场景插入空 signature。
- GLM 5.1 Anthropic-compatible streams 可能只在 `server_tool_use` 中发出一个 counter，因此 ProxAI 会把缺失的 `web_fetch_requests` 或 `web_search_requests` counter 填为 `0`。

不要添加其他 provider-specific 业务默认值，例如缺失 tool caller，除非有已测量的上游案例和聚焦 fixture 记录该行为。

这些修复应保持在 provider 兼容性处理中。它们不应重新定义官方 wire model。

## 文档维护期望

当协议转换或 wire-model 对齐规则发生变化时：

1. 更新本文档。
2. 如果行为变化影响用户或示例，更新 `docs/protocol-*.md` 下的相关协议文档。
3. 当变化影响面向用户的开发工作流或配置时，更新 `README.md` 和 `README_CN.md`。
