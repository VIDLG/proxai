# 错误处理与响应投影设计

本文说明 proxai 当前的错误处理模型，重点解释内部错误类型如何转换成客户端可见的 HTTP/SSE 错误响应，以及为什么需要 `ErrorResponseSpec`、`ErrorResponseFields`、`ErrorResponsePayload` 这几层结构。

## 设计目标

proxai 的错误处理有几个目标：

1. 内部错误保持强类型，便于代码分支和诊断。
2. 客户端错误响应保持稳定、紧凑、可读。
3. HTTP error response 和 SSE stream error event 复用同一套错误 payload。
4. 非 2xx upstream error 尽量保留有用信息，例如 `Retry-After`、`code`、`param`。
5. 不把内部错误 taxonomy 原样暴露给客户端。
6. 不把错误处理逻辑分散到 provider/upstream/translation 各处。

整体链路是：

```text
内部 typed error
  Error / RequestError / ConfigError / InternalError / UpstreamError / UpstreamResponseError
        ↓ error/render.rs
客户端响应 spec
  ErrorResponseSpec
        ↓
响应字段
  ErrorResponseFields
        ↓
客户端 payload
  ErrorResponsePayload
        ↓
HTTP text/json response 或 SSE error event bytes
```

## 内部错误类型

内部错误类型定义在 `src/error/` 下，另有 translation / SSE / stream 领域错误在各自模块中定义：

- `Error`
- `RequestError`
- `ConfigError`
- `InternalError`
- `UpstreamError`
- `UpstreamResponseError`
- `TranslationError`
- `SseError` / `SseTranslationError`
- `ByteStreamError`

这些类型描述的是 proxai 内部失败原因，而不是客户端响应格式。错误应按领域分层，不要用宽泛转换把语义错误包装成 `std::io::Error`；`io::Error` 只用于真实 OS / filesystem IO。

例如：

```rust
pub enum Error {
    Request(RequestError),
    Config(ConfigError),
    Internal(InternalError),
    Upstream(Box<UpstreamError>),
}
```

`UpstreamResponseError` 负责保存 upstream 非 2xx 响应体中解析出的紧凑错误信息：

```rust
Upstream {
    code: Option<String>,
    message: String,
    param: Option<serde_json::Value>,
}
```

这里的 `code` / `param` 是 upstream truth：只有上游实际提供时才保留，不再伪造默认 code。

## 响应投影边界：`error/render.rs`

所有 client-facing 错误响应投影集中在 `src/error/render.rs`。

这里的职责是把内部 typed error 转成客户端响应规格：

```rust
impl Error {
    pub(crate) fn response_spec(&self) -> ErrorResponseSpec { ... }
}
```

`Error::response_spec()` 只做顶层分发：

- `RequestError` → 400 `invalid_request_error`
- `ConfigError` / `InternalError` → 500 `internal_error`
- `UpstreamError` → 交给 `upstream_error_response_spec(...)` 私有函数处理

`UpstreamError` 本身不实现响应渲染方法，避免 domain error 类型承担 client response 职责。

## `ErrorResponseSpec`

`ErrorResponseSpec` 描述一个 HTTP error response 应该长什么样：

```rust
struct ErrorResponseSpec {
    fields: ErrorResponseFields,
    headers: HeaderMap,
}
```

它包含：

- `fields`：HTTP status 与错误 payload。
- `headers`：额外要附加到 HTTP response 的 headers。

普通错误的 `headers` 为空。

upstream 非 2xx 错误会从上游响应头中筛选可转发 header，例如 `Retry-After`：

```rust
ErrorResponseSpec::with_forwardable_headers(
    upstream_response_error_fields(head.status, parsed),
    &head.headers,
)
```

因此 `ErrorResponseSpec` 是 response 的规格，不是内部错误类型，也不是最终 `Response<Body>`。

最终渲染：

```rust
spec.into_response(format)
```

其中 `format` 来自配置：

- `text`
- `json`

## `ErrorResponseFields`

`ErrorResponseFields` 是 HTTP/SSE 共用的错误响应字段：

```rust
struct ErrorResponseFields {
    http_status: StatusCode,
    payload: ErrorResponsePayload,
}
```

`http_status` 是真正的 HTTP status line：

```http
HTTP/1.1 502 Bad Gateway
```

`payload` 是客户端可见的 JSON/SSE 错误内容。

注意：`ErrorResponseFields` 不是最终 HTTP response，因为它不包含额外 response headers；headers 在 `ErrorResponseSpec` 里。

## `ErrorResponsePayload`

`ErrorResponsePayload` 是客户端可见的错误 payload：

```rust
struct ErrorResponsePayload {
    message: String,
    error_type: ErrorResponseType,
    code: Option<String>,
    param: Option<Value>,
    status: u16,
}
```

序列化后的典型形状：

```json
{
  "message": "quota exhausted",
  "type": "upstream_error",
  "code": "rate_limit_exceeded",
  "param": "input",
  "status": 429
}
```

字段含义：

- `message`：人类可读错误信息。
- `type`：proxai client-facing 错误分类。
- `code`：上游提供的 OpenAI-style 错误 code，可选。
- `param`：上游提供的 OpenAI-style 错误 param，可选。
- `status`：数字状态码，给 JSON/SSE payload 使用。

`status` 使用 `u16`，不是 `StatusCode`，因为它是 wire payload 字段。真正的 HTTP status 使用 `ErrorResponseFields::http_status`。

SSE stream 已经开始后不能再修改 HTTP status line，所以 payload 里的数字 `status` 对 SSE error event 尤其重要。

## `ErrorResponseType`

`ErrorResponseType` 是 client-facing `type` 字段的强类型枚举：

```rust
#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
enum ErrorResponseType {
    InvalidRequestError,
    InternalError,
    UpstreamRequestError,
    UpstreamError,
    UpstreamResponseBodyReadError,
    UpstreamErrorBodyEmpty,
    UpstreamErrorBodyNonJson,
    UpstreamErrorBodyUnknownShape,
    SseTranslationError,
}
```

它序列化成：

```json
"invalid_request_error"
"internal_error"
"upstream_error"
```

这里没有使用裸字符串，是为了避免拼写错误。

这里也没有额外使用 `strum` 或 `derive_more`，因为当前唯一需求是 serde wire 序列化，`#[serde(rename_all = "snake_case")]` 已足够。

## HTTP error response

HTTP 错误响应由 `render_http_error_response(...)` 渲染。

### text 格式

配置为 text 时：

```http
Content-Type: text/plain; charset=utf-8

quota exhausted
```

body 只输出 `payload.message`。

### JSON 格式

配置为 JSON 时：

```json
{
  "error": {
    "message": "quota exhausted",
    "type": "upstream_error",
    "code": "rate_limit_exceeded",
    "param": "input",
    "status": 429
  }
}
```

外层 envelope 是：

```rust
struct ErrorJsonResponse {
    error: ErrorResponsePayload,
}
```

## SSE error event

SSE 错误事件由 `ErrorResponsePayload` 编码：

```rust
impl ErrorResponsePayload {
    fn encode_sse_event(self) -> io::Result<Bytes> { ... }
}
```

输出形状：

```text
event: error
data: {"type":"error","error":{...}}

```

对应 JSON data：

```json
{
  "type": "error",
  "error": {
    "message": "translation failed",
    "type": "sse_translation_error",
    "status": 502
  }
}
```

`ErrorResponseFields` 通过 `delegate!` 转发到 payload，保持调用方便：

```rust
ErrorResponseFields::sse_translation(...)
    .encode_sse_event()
```

但真正负责 SSE bytes 编码的是 `ErrorResponsePayload`。

### fallback SSE error

如果 typed SSE serialization 自身失败，会使用静态兜底 bytes：

```rust
FALLBACK_SSE_ERROR_EVENT
```

这个 fallback 故意不依赖 serde，避免在错误流已经异常时再次失败。

## Upstream error 的处理

upstream 错误分三类：

### 1. 请求发送失败

例如连接失败、DNS 失败：

```rust
UpstreamError::RequestSend(error)
```

映射成：

```json
{
  "message": "upstream request failed: ...",
  "type": "upstream_request_error",
  "status": 502
}
```

### 2. 上游返回非 2xx HTTP status

```rust
UpstreamError::ErrorStatus { head, parsed, .. }
```

使用上游 HTTP status，例如 429。

如果上游错误体是：

```json
{
  "error": {
    "message": "quota exhausted",
    "code": "rate_limit_exceeded",
    "param": "input"
  }
}
```

proxai 会输出：

```json
{
  "error": {
    "message": "quota exhausted",
    "type": "upstream_error",
    "code": "rate_limit_exceeded",
    "param": "input",
    "status": 429
  }
}
```

并保留可转发错误响应头，例如：

```http
Retry-After: 30
```

### 3. 已拿到响应头，但读取响应体失败

```rust
UpstreamError::ResponseBodyRead { source, .. }
```

映射成：

```json
{
  "message": "upstream response body read failed: ...",
  "type": "upstream_response_body_read_error",
  "status": 502
}
```

## Zed v1.5.3 兼容性

proxai 当前行为基于 `contrib/zed` v1.5.3 的实际解析逻辑。

### Responses API stream

Zed 的 Responses stream error 支持：

```json
{
  "type": "error",
  "message": "...",
  "code": "...",
  "param": "..."
}
```

也支持 nested 形状：

```json
{
  "type": "error",
  "error": {
    "message": "...",
    "code": "...",
    "param": "..."
  }
}
```

proxai 的通用 SSE error event 使用 nested 形状，Zed v1.5.3 可以解析。

OpenAI Responses stream 内部由 proxai 注入的错误，例如 tool argument stream 错误，会优先使用 top-level generic Responses error shape：

```json
{
  "type": "error",
  "sequence_number": 7,
  "code": null,
  "message": "tool stream stalled",
  "param": null
}
```

### Chat Completions stream

Zed 的 Chat Completions parser 只读取 `data:` 行，并接受：

```json
{
  "error": {
    "message": "..."
  }
}
```

proxai 的通用 SSE error data 中包含 nested `error.message`，因此也能被 Zed 读出 message。多余字段会被忽略。

## 为什么没有旧版 Zed flatten 兼容层

旧版 Zed 需要把：

```json
{
  "type": "error",
  "error": {
    "message": "..."
  }
}
```

展平成：

```json
{
  "type": "error",
  "message": "..."
}
```

但 Zed v1.5.3 已经同时支持 top-level 和 nested generic error payload。

因此 proxai 不再保留 legacy flatten compat 模块。

## 当前设计边界

当前错误响应相关结构的职责是：

```text
ErrorResponseSpec
  HTTP response spec: fields + extra headers

ErrorResponseFields
  HTTP/SSE 共用字段: http_status + payload

ErrorResponsePayload
  客户端 wire payload: message/type/code/param/status

ErrorResponseType
  强类型 client-facing type label

ErrorJsonResponse
  HTTP JSON envelope: { "error": payload }

ErrorSseEvent
  SSE envelope: { "type": "error", "error": payload }
```

这套结构避免了几类混淆：

- 内部错误类型不直接等于客户端响应类型。
- HTTP status line 和 payload numeric status 分开。
- HTTP response 渲染和 SSE event bytes 编码分开。
- 上游原始 `code` / `param` 不丢失，但也不会伪造。
- 额外 upstream headers 放在 response spec 中，而不是混进 payload。

## 验证重点

相关测试覆盖：

- 通用 SSE error 可被 Zed v1.5.3 Responses nested generic error parser 解析。
- 通用 SSE error 可被 Zed Chat Completions stream error parser 解析。
- upstream `code` / `param` 会进入 client-facing payload。
- OpenAI Responses stream 内部注入错误使用 Zed 可解析的 top-level generic error shape。

推荐验证命令：

```sh
just fmt_check
just test_lib
```
