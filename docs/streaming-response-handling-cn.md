# Provider Streaming Response 处理说明

本文说明 `proxai` 中三个 provider protocol 的 streaming response 处理流程：

- OpenAI Responses
- OpenAI Chat Completions
- Anthropic Messages

重点解释 provider response 层如何观察上游 SSE 字节流、维护协议状态、判断流结束/异常，并生成日志与诊断信息。

## 总体分层

streaming response 的处理大致分为三层：

```text
pipeline/upstream_response.rs
  判断上游 2xx response 是否为 SSE
  -> provider::handle_streaming_success_response(...)

provider/<protocol>/response/streaming.rs
  构造协议专属 BodyObserver
  调用 upstream::prepare_response_stream(...)
  返回保留原始响应语义的 outbound streaming body

upstream/streaming.rs
  包装 reqwest bytes_stream
  记录通用 stream 指标
  调用 BodyObserver 生命周期 hook
```

其中 `translation/` 不参与 provider response 观察和 provider-local normalization。provider streaming 处理仍然位于 `provider/` 边界内。

## 通用 streaming carrier

核心通用入口在：

```rust
src/upstream/streaming.rs
```

主要类型：

```rust
pub(crate) trait BodyObserver: Send + Unpin + 'static {
    fn on_chunk(&mut self, _chunk: &[u8]) -> BodyAction;
    fn on_stream_error(&mut self, error: &reqwest::Error);
    fn poll_pending_action(&mut self, _cx: &mut Context<'_>) -> BodyAction;
    fn on_stream_finished(&self, head: &UpstreamResponseHead, stats: UpstreamBodyStreamStats);
}
```

`MonitoredUpstreamBodyStream` 负责通用 stream carrier 行为：

1. 从 `reqwest::Response::bytes_stream()` 拉取 chunk。
2. 记录通用 upstream stream 指标。
3. 观察通用 upstream chunk 日志/捕获点。
4. 调用协议专属 `BodyObserver::on_chunk(...)`。
5. 处理 read idle timeout。
6. 在 EOF、错误、超时、注入错误 chunk 或 drop 时调用 `on_stream_finished(...)`。

`BodyAction` 表达协议 observer 是否需要干预输出：

```rust
pub(crate) enum BodyAction {
    Continue,
    InjectAndClose(Bytes),
}
```

大多数协议返回 `Continue`，只有 OpenAI Responses 的工具参数超时/异常会注入一条 SSE error 并关闭流。

## 三协议共同结构

三个协议的 streaming observer 现在基本一致：

```text
streaming.rs
  - 持有协议 State
  - 持有 SseEventScanner
  - on_chunk:
      1. scan chunk -> Vec<SseEvent>
      2. state.observe_events(&events)
      3. 做协议特有生命周期检查
  - on_stream_error: 记录 UpstreamStreamError
  - on_stream_finished: 生成 snapshot 并 emit outcome

state.rs
  - 持有协议状态
  - observe_events(&[SseEvent]) 将事件落入状态
  - 生成 summary / error / terminal 状态
```

也就是说：

- `streaming.rs` 负责 stream 生命周期和 carrier hook。
- `state.rs` 负责协议事件到状态的归纳。
- `SseEventScanner` 只在 observer 中持有，chunk 只扫描一次。
- 不再有额外的 `tracker.rs` wrapper。

## OpenAI Responses

主要文件：

```text
src/provider/openai/responses/response/streaming.rs
src/provider/openai/responses/response/state.rs
src/provider/openai/responses/response/state_events.rs
src/provider/openai/responses/response/tool_arguments.rs
```

observer 结构：

```rust
struct OpenaiResponsesUpstreamBodyObserver {
    state: ResponsesUpstreamState,
    recent_tail: Vec<u8>,
    saw_terminal_event: bool,
    stream_error: Option<UpstreamStreamError>,
    tool_arguments: ToolArgumentStreamState,
    timeout: Option<Duration>,
    sse_scanner: SseEventScanner,
    obs: ObserveContext,
}
```

### on_chunk 流程

```text
chunk
  -> 维护 recent_tail，最多保留 16 KiB
  -> SseEventScanner::scan(chunk)
  -> ResponsesUpstreamState::observe_events(&events)
  -> 检查 terminal event
  -> 检查 tool call arguments 是否异常/超时相关
```

`ResponsesUpstreamState::observe_events(...)` 位于 `state_events.rs`，负责：

- 解析 OpenAI Responses stream event。
- 记录最新 `sequence_number`。
- 记录 `response.created` / `response.completed` 等 snapshot。
- 记录 output item / function call / MCP 等增量观察状态。
- 识别某些 provider 返回的 nested generic error event。

### tool argument stall 处理

OpenAI Responses 有额外语义：工具调用参数可能开始后长时间没有完成。`ToolArgumentStreamState` 负责追踪这类状态。

如果检测到异常或 timeout：

1. observer 记录 `UpstreamStreamError::Stream`。
2. 构造一条 OpenAI Responses 风格 SSE error event。
3. 返回：

```rust
BodyAction::InjectAndClose(error_sse_chunk(...))
```

上游 stream carrier 会把该 error chunk 发给客户端并关闭流。

### unfinished tool diagnostics

OpenAI Responses observer 维护：

```rust
recent_tail: Vec<u8>
```

最多保留最近 16 KiB stream bytes。stream snapshot 中包含该 tail：

```rust
ResponsesUpstreamStreamSnapshot {
    head,
    metrics,
    state,
    recent_tail,
    metadata,
}
```

当 outcome 是 `UnfinishedTool` 时，diagnostics 会用 `snapshot.recent_tail` 生成本地诊断 JSON，用于分析最后的 SSE 尾部是否缺失 terminal event 或 arguments done event。

## OpenAI Chat Completions

主要文件：

```text
src/provider/openai/chat_completions/response/streaming.rs
src/provider/openai/chat_completions/response/state.rs
src/provider/openai/chat_completions/response/observed.rs
```

observer 结构：

```rust
struct ChatUpstreamBodyObserver {
    state: ChatUpstreamResponseState,
    sse_scanner: SseEventScanner,
    stream_error: Option<UpstreamStreamError>,
    obs: ObserveContext,
}
```

### on_chunk 流程

```text
chunk
  -> SseEventScanner::scan(chunk)
  -> ChatUpstreamResponseState::observe_events(&events)
```

`ChatUpstreamResponseState::observe_events(...)` 负责：

- 识别 `[DONE]` sentinel。
- 解析 `CreateChatCompletionStreamResponse`。
- 应用增量 observed updates。
- 记录 partial / terminal stream chunk projection。

Chat Completions 的 EOF 完整性判断很简单：

```rust
state.stream_done
```

只有看到 `[DONE]` sentinel 才认为 stream 完整完成。否则 EOF 会被记录为 `Closed`。

## Anthropic Messages

主要文件：

```text
src/provider/anthropic_messages/response/streaming.rs
src/provider/anthropic_messages/response/state.rs
src/provider/anthropic_messages/response/normalize/
```

observer 结构：

```rust
struct AnthropicSseObserver {
    state: AnthropicResponseState,
    stream_error: Option<UpstreamStreamError>,
    sse_scanner: SseEventScanner,
    obs: ObserveContext,
}
```

### on_chunk 流程

```text
chunk
  -> SseEventScanner::scan(chunk)
  -> AnthropicResponseState::observe_events(&events)
```

`AnthropicResponseState::observe_events(...)` 负责：

- 解析 SSE data JSON。
- 对 Anthropic-compatible provider 的 stream event payload 做 provider-local normalization。
- 解析为 `MessageStreamEvent`。
- 记录 message id / model / token usage / stop reason / stream_done / summary。

Anthropic stream 完整性判断：

```rust
state.stream_done()
```

也就是是否看到 `message_stop`。

### compatibility normalization

Anthropic-compatible provider 可能返回非严格 Anthropic Messages 形状。相关 normalization 位于：

```text
src/provider/anthropic_messages/response/normalize/
```

streaming response 中，如果 provider compatibility 是 `AnthropicCompatible`，outbound SSE body 会经过：

```rust
normalize::normalize_sse_stream(body_stream)
```

这属于 provider-local response normalization，不进入 `translation/`。

## outcome 语义

provider stream outcome 统一通过：

```rust
ProviderStreamOutcomeObserved {
    snapshot,
    outcome,
}
```

常见 outcome：

```rust
ProviderStreamOutcome::Completed
ProviderStreamOutcome::Closed
ProviderStreamOutcome::Error(...)
ProviderStreamOutcome::UnfinishedTool(...)
```

三协议判断方式：

| 协议 | Completed 条件 | 特殊错误 |
| --- | --- | --- |
| OpenAI Responses | 看到 Responses terminal event | tool arguments stall / unfinished tool |
| Chat Completions | 看到 `[DONE]` | 无协议特有注入 |
| Anthropic Messages | 看到 `message_stop` | 无协议特有注入 |

## 设计原则

这块代码遵循以下原则：

1. **stream carrier 和协议语义分离**
   - `upstream/streaming.rs` 只处理通用 bytes stream、timeout、metrics。
   - provider `streaming.rs` 处理协议级 SSE 观察。

2. **scanner 只在 observer 层**
   - chunk 只扫描一次。
   - state 只接收已经解码好的 `SseEvent`。

3. **state 只做事件归纳**
   - 不持有 scanner。
   - 不持有 HTTP response/body。
   - 不负责输出 body 转换。

4. **provider compatibility 留在 provider 层**
   - Anthropic normalization 不进入 translation。
   - Responses diagnostics 不伪装成三协议通用事件。

5. **诊断数据就近拥有**
   - OpenAI Responses recent tail 由 Responses observer 维护。
   - snapshot 携带诊断所需 tail，diagnostics 只负责写报告。
