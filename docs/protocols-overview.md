# proxai 协议模型总览

本文说明 proxai 内部如何理解“协议”。具体 wire shape 分别见：

- [OpenAI Responses 协议](protocol-openai-responses.md)
- [OpenAI Chat Completions 协议](protocol-openai-chat-completions.md)
- [Anthropic Messages 协议](protocol-anthropic-messages.md)

## 两条轴

proxai 把请求链路拆成两条独立的轴。

阶段轴描述数据在代理链路里的位置：

```text
inbound_request -> forwarded_request -> upstream_response -> outbound_response
```

协议轴描述这一阶段使用的 wire 协议：

```text
openai_responses / openai_chat_completions / anthropic_messages
```

对应到代码：

- `src/protocol/mod.rs` 定义 `RequestProtocol` 和 `ProviderProtocol`。
- `src/ingress/request.rs` 用 `PreparedInboundRequest` 按协议承载已经解析过的入站请求。
- `src/translation/request.rs` 根据入站协议和 provider 协议构造 `ForwardedRequest`。
- `src/provider/handler.rs` 根据 provider 协议选择上游响应处理器。

这个拆分避免把“客户端发来的协议”和“上游 provider 使用的协议”混成一个概念。例如客户端可以发 `openai_responses`，但路由到 `anthropic_messages` provider 时必须经过显式翻译；如果对应 pair 尚未实现，`translation::translate_request` 会返回明确错误。

## 当前运行时支持

当前代码的运行时路径支持：

- `openai_responses -> openai_responses`
- `openai_chat_completions -> openai_chat_completions`
- `anthropic_messages -> anthropic_messages`

跨协议转换目前保留为显式未实现路径。文档里描述的协议结构来自 `src/protocol/**/wire` 和各协议 projection；其中 Anthropic Messages 的完整 wire model 已在 `src/protocol/anthropic/messages` 建模，但部分结构仍是为后续翻译和更深观察预留的脚手架。

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
enum ForwardedRequest {
    OpenaiResponses(Box<OpenaiResponsesForwardedRequest>),
    OpenaiChatCompletions(Box<OpenaiChatCompletionsForwardedRequest>),
    AnthropicMessages(Box<AnthropicMessagesForwardedRequest>),
}
```

这让协议不可能和 payload/projection/summary 组合成非法状态。

## SSE 处理

SSE 流式响应分为两层：

- 通用层：`src/provider/stream.rs` 的 `log_upstream_body_stream` 保留上游原始 bytes，统计 chunk/bytes/duration，接入 capture，并调用协议 observer。
- 协议层：各 provider observer 解析或扫描本协议事件，识别终止事件，产生日志和必要诊断。

proxai 的默认目标不是重新生成流，而是尽量保留上游原始 SSE bytes 和 `text/event-stream`。只有协议确实需要诊断或错误注入时，observer 才会在 `poll_pending` 中产出自定义 chunk，例如 OpenAI Responses 的工具参数流超时诊断。

## 工具调用的两类来源

本文把工具调用分成两类：

- 用户端工具调用：模型请求客户端或调用方执行的工具，例如 OpenAI `function` tool、Chat Completions `tool_calls`、Anthropic `tool_use`。结果通常由下一轮请求带回。
- 服务端工具调用：由上游模型服务或其托管环境执行的工具，例如 OpenAI Responses 的 web/file/code/MCP 等 hosted tool 事件，Anthropic 的 `server_tool_use` 和 server-tool result blocks。

不同协议对这两类工具的表示方式不同，这也是 proxai 保持协议专属 wire model 的原因。
