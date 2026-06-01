# Provider 基础设施重构计划

这份文档用于记录 provider/protocol 基础设施的设计方向，避免后续开发中遗忘上下文或走回头路。

## 核心设计原则：协议优先的 enum

对于带有协议语义的请求/响应数据，优先使用“顶层按协议分支的 enum”，每个 variant 包住对应协议自己的具体结构。

推荐：

```rust
enum PreparedInboundRequest {
    OpenaiResponses(openai_responses::PreparedRequest),
    OpenaiChatCompletions(openai_chat_completions::PreparedRequest),
    AnthropicMessages(anthropic_messages::PreparedRequest),
}
```

避免：

```rust
struct PreparedInboundRequest {
    protocol: RequestProtocol,
    payload: InboundRequestPayload,
    projection: ProjectionEnum,
    summary: SummaryEnum,
}
```

后者的问题是多个平行字段之间存在隐含一致性约束，比如可能出现 `protocol = AnthropicMessages`，但 payload 却是 OpenAI Responses 的非法状态。协议优先的 enum 可以从类型层面避免这类 impossible state。

如果通用 pipeline 需要协议无关信息，可以在 enum 上提供 accessor：

```rust
impl PreparedInboundRequest {
    fn protocol(&self) -> RequestProtocol;
    fn model(&self) -> &str;
}
```

这个原则适用于协议相关的语义数据：

- prepared inbound request
- forwarded request envelope
- forwarded request log view
- upstream response envelope / observer
- outbound response envelope
- 各协议自己的 request/response projection 和 summary

不要把这个原则强行套到通用基础设施上。以下结构仍然适合普通 struct：

- `ProviderRuntime`
- `UpstreamResponseHead`
- `ContentType`
- `ErrorInfo`
- `CaptureSession`
- `CaptureDestination`

## 阶段轴和协议轴分离

阶段轴：

```text
inbound_request -> forwarded_request -> upstream_response -> outbound_response
```

协议轴：

```text
openai_responses / openai_chat_completions / anthropic_messages
```

两条轴不要混成一个概念。

推荐理解：

- `inbound_request.protocol`：客户端发来的协议。
- `forwarded_request.protocol`：proxai 发给上游的协议。
- `upstream_response.protocol`：上游返回的协议。
- `outbound_response.protocol`：proxai 返回给客户端的协议。

路由和翻译层负责连接两条轴，但命名上要保持清楚。

## 目标 pipeline

长期目标是让主流程接近：

```text
raw request body
  -> ingress::prepare_inbound_request
  -> routing::resolve_route
  -> translation::translate_request
  -> provider transport
  -> provider upstream response handling
  -> translation::translate_response（需要跨协议时）
  -> outbound response
```

`lib.rs` 应该只负责 orchestration，不应该长期持有 OpenAI Responses 专用的 request preparation、translation、upstream response 细节。

## 模块职责

```text
src/protocol/     wire model，以及协议原生 projection/summary
src/ingress/      inbound 协议解析、校验、归一化
src/translation/  显式的协议到协议转换
src/provider/     provider transport、headers/auth、上游响应处理、provider 通用基础设施
src/capture/      capture 生命周期和 artifacts
src/logging/      紧凑的、协议感知的日志 render view
```

跨协议转换应放在 `translation/`。不要把通用协议转换藏进 provider 模块，除非它确实是某个 provider 私有的 wire quirk。

## 当前阶段计划

### Phase 1：抽取通用 provider 基础设施

状态：基本完成。

已完成：

- `protocol::ErrorInfo` 替代 OpenAI Responses 专属的 `ErrorObject`，用于共享错误信息。
- `provider::error::normalize_upstream_error_body` 解析通用 upstream 非 2xx error body。
- `provider::response::UpstreamResponseHead` 和 `ContentType` 成为 provider 通用基础设施。
- OpenAI Responses stream/tool-call 专属错误仍保留在 `provider/openai/responses`。
- `RateLimit` 和 `CodexLimits` 暂时仍保留为 OpenAI Responses 专属结构。

### Phase 2：抽取 request pipeline

状态：基本完成。

目标：

```text
raw body
  -> PreparedInboundRequest enum
  -> ForwardedRequestEnvelope enum
  -> provider send
```

目标形态：

```rust
enum PreparedInboundRequest {
    OpenaiResponses(openai_responses::PreparedRequest),
    OpenaiChatCompletions(openai_chat_completions::PreparedRequest),
    AnthropicMessages(anthropic_messages::PreparedRequest),
}
```

```rust
enum ForwardedRequestEnvelope {
    OpenaiResponses(openai_responses::PreparedForwardedRequest),
    OpenaiChatCompletions(openai_chat_completions::PreparedForwardedRequest),
    AnthropicMessages(anthropic_messages::PreparedForwardedRequest),
}
```

当前实现已支持：

```text
openai_responses -> openai_responses
openai_chat_completions -> openai_chat_completions
```

`ForwardedRequestEnvelope` / `ForwardedRequestLogView` / `ForwardedRequestCaptureView` 已移动到 `protocol` 层，并由 translation 构造、provider/logging 消费。未实现的转换路径应显式失败，并返回简洁错误。

### Phase 3：OpenAI Chat Completions no-conversion 路径

状态：基本完成。

目标：先支持：

```text
openai_chat_completions -> openai_chat_completions
```

先不做跨协议转换。

已新增：

```text
src/protocol/openai/chat_completions/
src/ingress/openai_chat_completions/
src/provider/openai/chat_completions/
```

当前 Chat Completions 路径使用 `async-openai` typed parse，支持 request projection / summary、HTTP `/v1/chat/completions` 路由、no-conversion forwarding、provider auth、response tracker、SSE passthrough、JSON/SSE usage/finish_reason 观察，以及 E2E/单元测试覆盖。

### Phase 4：Anthropic Messages no-conversion 路径

目标：先支持：

```text
anthropic_messages -> anthropic_messages
```

先不做跨协议转换。

预期新增：

```text
src/protocol/anthropic/messages/
src/ingress/anthropic_messages/
src/provider/anthropic/messages/
```

### Phase 5：跨协议转换

按真实需求逐步补 pair-oriented conversion。优先级暂定：

1. `openai_responses -> anthropic_messages`
2. `anthropic_messages -> openai_responses`
3. `openai_chat_completions -> openai_responses`

模块保持成对命名，例如：

```text
translation/openai_responses/to_anthropic_messages.rs
translation/anthropic_messages/to_openai_responses.rs
```

## Response / Streaming 方向

stream handling 分两层：通用 stream mechanics 和协议专属 observer。

通用 provider stream 负责：

- 保留 upstream 原始 bytes
- 统计 chunks / bytes / duration / average chunk size
- 集成 capture writer
- 处理基础 closed / completed / error 生命周期
- 在 `Poll::Pending` 时允许协议 observer 产生自定义 chunk，例如 Responses tool-call timeout SSE error

协议专属 observer 负责：

- 解析协议 stream events
- 构建协议专属 summary
- 识别协议专属 terminal event
- 处理 OpenAI Responses tool-call semantic timeout / unfinished-tool diagnostics

在多个具体协议路径真正出现之前，优先使用 enum dispatch，不要急着设计大型 generic trait hierarchy。

当前实现已经抽出：

```rust
trait UpstreamBodyObserver {
    fn observe_chunk(&mut self, chunk: &[u8]);
    fn finish(&mut self);
    fn poll_pending(...) -> UpstreamBodyObserverPoll;
    fn emit_completed(...);
    fn emit_closed(...);
    fn emit_error(...);
}
```

OpenAI Responses 和 OpenAI Chat Completions 都已迁到这套通用 stream mechanics。协议差异集中在各自 observer 内：Responses 保留 tool-call semantic timeout / unfinished-tool diagnostics；Chat Completions 负责解析 JSON response 和 `chat.completion.chunk` SSE。

## 非目标

- 不把 proxai 做成泛化多租户 AI gateway。
- 不把每个配置项都暴露成 CLI flag。
- 不把通用协议转换藏在 provider 模块里。
- 不在具体协议路径出现前过度抽象出大型 generic protocol traits。
- 不让 provider name 承载协议语义；provider name 仍然只是用户定义 label。
