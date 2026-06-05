# proxai 协议模型总览

本文说明 proxai 内部如何理解“协议”。具体 wire shape 分别见：

- [OpenAI Responses 协议](protocol-openai-responses.md)
- [OpenAI Chat Completions 协议](protocol-openai-chat-completions.md)
- [Anthropic Messages 协议](protocol-anthropic-messages.md)

## 两条轴

proxai 把请求链路拆成两条独立的轴。

阶段轴描述数据在代理链路里的位置：

```text
inbound_request -> provider_request -> upstream_response -> outbound_response
```

协议轴描述这一阶段使用的 wire 协议：

```text
openai_responses / openai_chat_completions / anthropic_messages
```

对应到代码：

- `src/protocol/mod.rs` 定义 `RequestProtocol` 和 `ProviderProtocol`。
- `src/ingress/request.rs` 用 `PreparedInboundRequest` 按协议承载已经解析过的入站请求。
- `src/translation/request.rs` 把已经规范化的入站 payload 翻译成 provider 协议 payload。
- `src/provider/request.rs` 准备 provider 请求，包括模型改写、projection/summary 提取和 JSON body 序列化。
- `src/provider/response.rs` 以及 provider response 模块按 provider 协议选择上游响应处理方式。

这个拆分避免把“客户端发来的协议”和“上游 provider 使用的协议”混成一个概念。例如客户端可以发 `openai_responses`，但路由到 `anthropic_messages` provider 时必须经过显式翻译；如果对应 pair 尚未实现，`translation::translate_request` 会返回明确错误。

## 当前运行时支持

当前代码的运行时路径支持：

- `openai_responses -> openai_responses`
- `openai_chat_completions -> openai_chat_completions`
- `anthropic_messages -> anthropic_messages`
- `openai_responses -> openai_chat_completions`
- `openai_responses -> anthropic_messages`
- `openai_chat_completions -> anthropic_messages`
- `anthropic_messages -> openai_responses`

其他跨协议转换保持显式未支持，直到逐个实现。文档里描述的协议结构来自 `src/protocol/**` 和各协议 projection；其中 Anthropic Messages 的完整 wire model 已在 `src/protocol/anthropic/messages` 建模，部分结构也用于翻译和观察。

## Chat Completions choices 与 Responses output items

Chat Completions 和 Responses 都需要表达“一个响应里可能有多个并列输出单元，并且每个输出单元内部按时间串行地产生 delta”。两者的核心差异是输出单元的粒度不同。

Chat Completions 是 `choice/message` 中心：

```text
response
└─ choices: Vec<Choice>              // 并列候选回答
   ├─ choice[0]
   │  ├─ delta.content 串行到达
   │  └─ delta.tool_calls[index] 参数串行到达
   └─ choice[1]
      └─ delta.content 串行到达
```

Responses 是 `output item` 中心，更扁平：

```text
response
└─ output: Vec<OutputItem>           // 并列输出实体
   ├─ output[0] reasoning rs_1
   │  └─ summary[0] delta 串行到达
   ├─ output[1] message msg_1
   │  └─ content[0] text delta 串行到达
   └─ output[2] function_call fc_1
      └─ arguments delta 串行到达
```

可以把两者统一理解为：

```text
并行维度 = 外层 key 有几个
串行维度 = 同一个 key 下的 delta 按到达顺序拼接
```

Chat Completions 的并行 key：

- `choice.index` 区分不同候选回答。
- `choice.index + tool_call.index` 区分同一 choice 内的不同工具调用。
- `tool_call.id` 是有用的辅助信息，但 stream 后续 delta 不一定每次携带，所以不能只靠 id 聚合。

Responses 的并行 key：

- 优先使用 output item 的 `id` / 事件里的 `item_id`。
- 没有稳定 id 时，用 `kind + output_index` 作为 fallback。
- item 内部再用 `content_index` / `summary_index` 区分多个内容 part。

同一个“并行读取两个文件”的例子：

Chat Completions 表达为一个 choice message 里的多个 tool calls：

```text
choices[0].message.tool_calls = [
  read_file(src/main.rs),
  read_file(Cargo.toml)
]
```

Responses 表达为顶层 output 里的多个 function_call item：

```text
output = [
  function_call read_file(src/main.rs),
  function_call read_file(Cargo.toml)
]
```

所以可以概括为：

```text
Chat Completions: choice -> message -> tool_calls
Responses:        output -> item
```

Responses 不是完全没有嵌套，`message.content[]`、`reasoning.summary[]` 仍然存在；但主要输出实体被提升为顶层 `output[]` item，因此比 Chat Completions 更 flat。

这也是 proxai observer state 的建模依据：

- Chat Completions observer 按 `choice.index` 和 `tool_call.index` 去重 stream delta。
- Responses observer 按 `item_id` / `output_index` 去重 output item 生命周期事件。
- 当前 observer 主要记录实体数量、名称、finish reason 和错误摘要；如果未来需要重建完整文本或参数，需要为同一 key 下的串行 delta 维护 buffer。

## 数据结构分层

协议相关数据优先使用“顶层 enum 按协议分支”的结构，而不是一个通用 struct 里放多个可漂移字段。

入站请求：

```rust
enum PreparedInboundRequest {
    OpenaiResponses(PreparedOpenaiResponsesRequest),
    OpenaiChatCompletions(PreparedOpenaiChatCompletionsRequest),
    AnthropicMessages(PreparedAnthropicMessagesRequest),
}
```

转发请求：

```rust
enum ProviderRequest {
    OpenaiResponses(Box<OpenaiResponsesProviderRequest>),
    OpenaiChatCompletions(Box<OpenaiChatCompletionsProviderRequest>),
    AnthropicMessages(Box<AnthropicMessagesProviderRequest>),
}
```

这让协议不可能和 payload/projection/summary 组合成非法状态。

## SSE 处理

SSE 流式响应分为三层：

- carrier 层：`http_support::ByteStream` / `ByteStreamError` 承载 boxed byte streams；`http_support::response` 负责重建 `text/event-stream` 响应头和 body。
- 通用上游层：`src/upstream/streaming.rs` 保留上游原始 bytes，统计 chunk/bytes/duration，接入 capture，并调用协议 `BodyObserver` 生命周期 hook。
- 协议层：各 provider observer 或 `src/translation/sse.rs` 解析/翻译本协议事件，识别终止事件，产生日志和必要诊断。

proxai 的默认目标不是重新生成流，而是尽量保留上游原始 SSE bytes 和 `text/event-stream`。只有协议确实需要诊断或错误注入时，observer 才会在 `poll_pending_action` 中产出自定义 chunk，例如 OpenAI Responses 的工具参数流超时诊断。

三协议 provider streaming response 的详细处理流程见 `docs/streaming-response-handling-cn.md`。

## 工具调用的两类来源

本文把工具调用分成两类：

- 用户端工具调用：模型请求客户端或调用方执行的工具，例如 OpenAI `function` tool、Chat Completions `tool_calls`、Anthropic `tool_use`。结果通常由下一轮请求带回。
- 服务端工具调用：由上游模型服务或其托管环境执行的工具，例如 OpenAI Responses 的 web/file/code/MCP 等 hosted tool 事件，Anthropic 的 `server_tool_use` 和 server-tool result blocks。

不同协议对这两类工具的表示方式不同，这也是 proxai 保持协议专属 wire model 的原因。
