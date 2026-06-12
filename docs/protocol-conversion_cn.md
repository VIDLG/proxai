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

## OpenAI Chat ↔ Anthropic Messages 消息位置

OpenAI Chat Completions 和 Anthropic Messages 都把对话表示为有序 turn，但 system 指令、工具调用和工具结果所在的位置不同。转换代码应显式保留这些位置规则。

### 总体位置对照

| 概念 | OpenAI Chat Completions | Anthropic Messages |
| --- | --- | --- |
| System 指令 | `messages[]` 中 `role: "system"` 的 item | 顶层 `system` 字段 |
| Developer 指令 | `messages[]` 中 `role: "developer"` 的 item | 没有专门 role；折叠进顶层 `system` |
| 用户内容 | `messages[]` 中 `role: "user"` 的 item | `messages[]` 中 `role: "user"` 的 item |
| Assistant 文本 | `messages[]` 中 `role: "assistant"` 且带 `content` | `messages[]` 中 `role: "assistant"` 且包含 text content blocks |
| 工具调用请求 | assistant message 的 `tool_calls[]` | assistant message content block，`type: "tool_use"` |
| 工具调用结果 | 独立的 `messages[]` item，`role: "tool"`，带 `tool_call_id` | user message content block，`type: "tool_result"` |
| 旧版 function 结果 | 独立的 `messages[]` item，`role: "function"` | 不支持；没有 `tool_call_id` 时无法可靠映射到 `tool_result` |

### System 与 developer 指令

Chat 把 system-like 指令放在有序 `messages[]` 数组中：

```json
{"role": "system", "content": "You are concise."}
{"role": "developer", "content": "Prefer exact answers."}
```

Anthropic 没有 `developer` role，也不会把 system 指令放进 `messages[]`。将 Chat `system` 和 `developer` content 转为 Anthropic 顶层 `system` 字段。只有一个非空文本片段时使用 string 形态；多个片段时使用 block 形态保留边界：

```json
{
  "system": [
    {"type": "text", "text": "You are concise."},
    {"type": "text", "text": "Prefer exact answers."}
  ]
}
```

### 用户内容

Chat `role: "user"` content 不包含工具结果。它包含普通用户输入的 content parts，例如文本、图片、音频或文件：

```json
{
  "role": "user",
  "content": [
    {"type": "text", "text": "Summarize this."},
    {"type": "image_url", "image_url": {"url": "https://example.test/a.png"}}
  ]
}
```

当目标协议能表达来源内容时，将它们转为 Anthropic user `content` 中的 text/image/document blocks。不支持的 user part 应返回 `TranslationError::InvalidPayload`，不要静默丢弃。

### 工具调用请求

在 Chat Completions 中，模型通过 assistant message 的 `tool_calls[]` 请求执行工具：

```json
{
  "role": "assistant",
  "content": "I will look that up.",
  "tool_calls": [
    {
      "id": "call_1",
      "type": "function",
      "function": {
        "name": "lookup",
        "arguments": "{\"query\":\"proxai\"}"
      }
    }
  ]
}
```

在 Anthropic Messages 中，相同请求是 assistant content block：

```json
{
  "role": "assistant",
  "content": [
    {"type": "text", "text": "I will look that up."},
    {
      "type": "tool_use",
      "id": "call_1",
      "name": "lookup",
      "input": {"query": "proxai"}
    }
  ]
}
```

Chat function tool arguments 是 JSON 字符串。转换为 Anthropic `tool_use.input` 时应解析为 JSON；如果无效，应让转换失败，不要用 `{}` 替代。

Chat function tools 可以映射到 Anthropic custom tools，因为二者都携带具名 JSON-schema 输入约束。Chat custom tools 不同：它们的输入是 freeform text 或 grammar-constrained text，不是由 `input_schema` 描述的 JSON object。转换到 Anthropic Messages 时应拒绝 Chat custom tool 定义、custom tool choice 和 custom tool call，而不是假装它们是空 object schema 的 JSON 工具。

### 工具调用结果

在 Chat Completions 中，工具执行输出不是 assistant message 的一部分，而是独立的 `role: "tool"` message：

```json
{
  "role": "tool",
  "tool_call_id": "call_1",
  "content": "found"
}
```

在 Anthropic Messages 中，工具结果是 user 侧 content block，并引用前面的 `tool_use.id`：

```json
{
  "role": "user",
  "content": [
    {
      "type": "tool_result",
      "tool_use_id": "call_1",
      "content": "found",
      "is_error": false
    }
  ]
}
```

因此 Chat `role: "tool"` message 会转成 Anthropic `role: "user"` message，其中包含 `tool_result` block。不要把 Chat 工具结果放进 Chat user content，也不要放进 Anthropic assistant content。

### 旧版 function message

Chat 除了现代 `tool_calls`，还有旧版 function-calling shape。转换到 Anthropic Messages 时应拒绝 `role: "function"` message。旧版 function 结果只有 function name，没有稳定的 `tool_call_id`；而 Anthropic `tool_result` 必须引用前面的 `tool_use.id`。不要发明 id，也不要把结果降级成普通 user text。

### 响应 choices 与候选回复

Chat Completions 响应中的 `choices[]` 是一组可替代的候选 assistant 回复，通常由请求参数 `n` 产生。它不是 content block 列表，也不是并行工具调用的表达方式。

```json
{
  "choices": [
    {"index": 0, "message": {"role": "assistant", "content": "方案 A"}},
    {"index": 1, "message": {"role": "assistant", "content": "方案 B"}}
  ]
}
```

并行工具调用位于单个候选 assistant message 内部，即
`choices[i].message.tool_calls[]`；这些工具调用可以映射为同一个 Anthropic
assistant message 中的多个 `tool_use` block。

Anthropic Messages 没有等价的顶层候选列表响应结构。非流式 Anthropic 响应是一个
`Message`，其中包含一条 `content[]` 序列，而不是多个可替代 assistant
message 的列表。OpenAI Responses API 也没有 Chat 风格的 `choices[]` 等价结构：
它的 `output[]` 是输出 item 序列（message、function call、reasoning item 等），
不是候选答案集合。

不要把多个 Chat choices 合并进一个 Anthropic `content[]` 数组，也不要静默只保留第一个 choice。这两种做法都会丢失协议语义：每个 choice 的 `index`、独立的 `finish_reason`，以及这些 choices 是互斥候选而不是同一个 assistant turn 的事实。将 Chat response 转为 Anthropic Messages 时，应要求恰好一个 choice，并拒绝 multi-choice response。

### Chat -> Anthropic 响应与流式语义

Chat -> Anthropic 非流式响应转换规则：

- 将 `choices[0].message.content` 映射为 Anthropic `text` blocks；
- 将 function `tool_calls[]` 映射为 Anthropic `tool_use` blocks，并把 Chat
  function `arguments` 解析为 JSON 后写入 `tool_use.input`；
- 当存在 `message.refusal` 时，将可见拒答文字保留为 `text` block，同时设置
  `stop_reason: "refusal"` 和 `stop_details.explanation`；Chat 没有 refusal
  category，因此不发明 category；
- 要求恰好一个 Chat choice，并拒绝没有可表示 text、refusal 或 function tool
  calls 的响应。

Chat -> Anthropic 流式转换应保持显式 lifecycle：

1. 等到第一个 assistant choice chunk 后再发 Anthropic `message_start`；
2. 将 Chat `delta.content` / `delta.refusal` 转为 Anthropic text block；第一段
   text 可以放在 `content_block_start` 中，后续片段使用 `text_delta`；
3. 将 Chat function tool-call start 转为 `tool_use` block start，并使用空 object
   `input`，因为 Chat streaming 的 `function.arguments` 是 partial JSON 字符串；
   这些参数片段通过 `input_json_delta` 发送；
4. 当 Chat `finish_reason` 到达时，关闭所有打开的 content blocks，并保存包含
   finish reason 和 refusal 文字的 pending terminal state；
5. 当后续到达 `choices: []` usage-only chunk，或 `[DONE]` / EOF 结束 stream 且没有
   final usage 时，输出 Anthropic `message_delta` / `message_stop`。

OpenAI 在设置 `stream_options: {"include_usage": true}` 后，最终 streaming usage
由最后一个 `choices: []` chunk 表达。转换时应把这个 usage-only chunk 作为 final
usage 来源。不要把非空 `choices` chunk 上的 `usage` 视为 final usage，也不要用它来
停止 Anthropic stream。一些 OpenAI-compatible 服务会在普通 chunk 上暴露
continuous/intermediate usage stats；这些值不能替代最终 usage-only chunk，本转换会忽略它们。

Chat stream 中的 `choices: []` chunk 只有在已经收到 terminal `finish_reason` 后，
作为 usage-only chunk 才是合法的。应拒绝出现在任何 assistant message 之前、
terminal finish reason 之前、或 Anthropic message stopped 之后的 usage-only chunk。
对 Chat stream `logprobs`、非 assistant delta role、multi-choice chunks，也应报错，
不要静默丢弃 Anthropic Messages 无法表示的信息。

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

Anthropic 没有专门的 refusal content block。可见拒答文字仍然是普通的 `content[]` text；拒答语义由 message 级 stop 字段携带：

- 可见文字：`content[]` 中的 `text` block；
- 拒答标记：`stop_reason: "refusal"`；
- 可选拒答元信息：`stop_details`，例如 `explanation` 和 provider 分类。

这和 Chat Completions 不同。Chat 把拒答文字放在普通内容旁边的同级字段 `choices[].message.refusal` 中，`message.content` 和 `message.refusal` 是两个不同的内容槽位。Anthropic 中，普通 assistant 文本和可见拒答文字使用同一种 `text` block shape；translator 只能通过 message 级 stop 字段判断这些 text block 应变成 Chat `message.content` 还是 Chat `message.refusal`。

拒答由 message 级字段识别：

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

- 将 Anthropic `thinking` block 文本和 `thinking_delta` 片段映射到 Zed 支持的 Chat-compatible 扩展字段 `delta.reasoning_content`；不要把 thinking 文本放进普通 `delta.content`；
- 忽略 `signature_delta` 和 `redacted_thinking` payload，不把它们泄露进 Chat content，因为 Chat Completions 没有标准且安全的字段承载这些值；
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
