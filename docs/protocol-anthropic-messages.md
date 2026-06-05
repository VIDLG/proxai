# Anthropic Messages 协议

proxai 中的协议名是 `anthropic_messages`，主要类型位于：

- `src/protocol/anthropic/messages/wire`
- `src/provider/anthropic_messages`

当前运行时已支持 `anthropic_messages -> anthropic_messages` 透传。完整 wire model 已在协议层建模，协议转换和 provider 兼容处理仍应保持显式、可测试。

## 请求模型

Anthropic Messages 请求主体是 `MessageCreateParamsBase`：

```rust
struct MessageCreateParamsBase {
    max_tokens: u32,
    messages: Vec<MessageParam>,
    model: String,
    stream: Option<bool>,
    system: Option<SystemPrompt>,
    tools: Option<Vec<ToolUnion>>,
    tool_choice: Option<ToolChoice>,
    thinking: Option<ThinkingConfigParam>,
    temperature: Option<Number>,
    top_k: Option<u32>,
    top_p: Option<Number>,
    stop_sequences: Option<Vec<String>>,
    ...
}
```

运行时 ingress 目前只要求 body 是 JSON 且包含非空 `model`，然后保留原始 body：

```rust
struct PreparedAnthropicMessagesRequest {
    body: Vec<u8>,
    model: String,
}
```

当前 wire request body 直接由 `MessageCreateParamsBase` 表示；不再额外保留薄封装 projection 类型。

Anthropic 的 `system` 不在 `messages` 数组里，而是单独字段：

```rust
enum SystemPrompt {
    Text(String),
    Blocks(Vec<TypedTextBlockParam>),
}
```

消息由 `MessageParam` 表示：

```rust
struct MessageParam {
    content: MessageParamContent,
    role: Role,
}

enum MessageParamContent {
    Text(String),
    Blocks(Vec<ContentBlockParam>),
}
```

`ContentBlockParam` 是请求侧 block union，包含文本、图片、文档、搜索结果、思考块、工具调用、工具结果、server tool 相关结果等。

## 非流式响应

Anthropic 非流式响应由 `Message` 表示：

```rust
struct Message {
    id: String,
    content: Vec<ContentBlock>,
    model: String,
    role: Role,
    type: MessageType,
    stop_reason: Option<StopReason>,
    stop_sequence: Option<String>,
    usage: Usage,
    ...
}
```

响应内容是 `ContentBlock`：

```rust
enum ContentBlock {
    Text(TextBlock),
    Thinking(ThinkingBlock),
    RedactedThinking(RedactedThinkingBlock),
    ToolUse(ToolUseBlock),
    ServerToolUse(ServerToolUseBlock),
    WebSearchToolResult(WebSearchToolResultBlock),
    WebFetchToolResult(WebFetchToolResultBlock),
    CodeExecutionToolResult(CodeExecutionToolResultBlock),
    BashCodeExecutionToolResult(BashCodeExecutionToolResultBlock),
    TextEditorCodeExecutionToolResult(TextEditorCodeExecutionToolResultBlock),
    ToolSearchToolResult(ToolSearchToolResultBlock),
    ContainerUpload(ContainerUploadBlock),
}
```

这里的重点是 Anthropic 把模型输出组织为 content blocks，而不是 OpenAI Responses 的 output items 或 Chat Completions 的单个 assistant message。

## 用户端工具调用

Anthropic 的用户端工具定义是 custom `Tool`：

```rust
struct Tool {
    name: String,
    description: Option<String>,
    input_schema: InputSchema,
    strict: Option<bool>,
    cache_control: Option<CacheControlEphemeral>,
    ...
}
```

模型请求客户端执行工具时，响应内容里出现 `ContentBlock::ToolUse`：

```rust
struct ToolUseBlock {
    id: String,
    caller: ToolCaller,
    input: Value,
    name: String,
}
```

客户端执行后，在下一轮请求里用 `ContentBlockParam::ToolResult` 回填：

```rust
struct ToolResultBlockParam {
    tool_use_id: String,
    content: Option<ToolResultContentParam>,
    is_error: Option<bool>,
    cache_control: Option<CacheControlEphemeral>,
}
```

关联字段是 `tool_use_id`，它对应上一轮 `ToolUseBlock.id`。`is_error` 用于表达客户端工具执行失败。

`tool_choice` 控制模型工具选择：

```rust
enum ToolChoice {
    Auto(ToolChoiceAuto),
    Any(ToolChoiceAny),
    Tool(ToolChoiceTool),
    None(ToolChoiceNone),
}
```

`Auto`、`Any`、`Tool` 支持 `disable_parallel_tool_use`，用于限制并行工具调用。

## 服务端工具调用

Anthropic 的协议模型显式区分 server tools。请求侧 `ToolUnion` 可以包含自定义工具和多种服务端工具，例如：

- `bash_20250124`
- `code_execution_20250522`
- `code_execution_20250825`
- `code_execution_20260120`
- `memory_20250818`
- `text_editor_*`
- `web_search_*`
- `web_fetch_*`
- `tool_search_*`

服务端工具调用在响应内容中表现为 `ServerToolUseBlock`：

```rust
struct ServerToolUseBlock {
    id: String,
    caller: ToolCaller,
    input: Value,
    name: ServerToolName,
}
```

服务端工具结果通过专门的 result block 返回：

```rust
struct ServerToolResultBlock {
    id: String,
    caller: ToolCaller,
    input: Value,
    name: ServerToolName,
    content: ServerToolResultContent,
    error: Option<ToolErrorBlock>,
    type: String,
}
```

`ServerToolName` 覆盖 web、code execution、bash、text editor、tool search 等服务端能力。

`ToolCaller` 表示是谁触发了这个工具：

```rust
enum ToolCaller {
    Direct(DirectCaller),
    CodeExecution20250825(ServerToolCaller),
    CodeExecution20260120(ServerToolCaller20260120),
}
```

这允许协议表达“直接由模型调用”和“由某个 code execution 工具继续调用其他 server tool”的区别。

## SSE 流式

Anthropic Messages 流事件由 `MessageStreamEvent` 表示：

```rust
enum MessageStreamEvent {
    Ping(PingEvent),
    MessageStart(MessageStartEvent),
    MessageDelta(MessageDeltaEvent),
    MessageStop(MessageStopEvent),
    ContentBlockStart(ContentBlockStartEvent),
    ContentBlockDelta(ContentBlockDeltaEvent),
    ContentBlockStop(ContentBlockStopEvent),
}
```

典型事件顺序：

```text
message_start
content_block_start
content_block_delta*
content_block_stop
message_delta
message_stop
```

`ping` 是保活事件，不改变消息内容。

内容增量由 `ContentBlockDelta` 表示：

```rust
enum ContentBlockDelta {
    TextDelta(TextDelta),
    InputJsonDelta(InputJsonDelta),
    CitationsDelta(CitationsDelta),
    ThinkingDelta(ThinkingDelta),
    SignatureDelta(SignatureDelta),
}
```

文本流使用 `TextDelta.text`。工具输入流通常使用 `InputJsonDelta.partial_json`，客户端需要按 content block `index` 聚合 partial JSON。思考内容使用 `ThinkingDelta`，签名使用 `SignatureDelta`。

block 通过 `index` 关联：

- `ContentBlockStartEvent.index`：开始一个 content block。
- `ContentBlockDeltaEvent.index`：给对应 block 增量追加内容。
- `ContentBlockStopEvent.index`：该 block 结束。

消息级结束由 `MessageStopEvent` 表示；`MessageDeltaEvent` 携带 `stop_reason`、`stop_sequence` 和增量 usage。

## content blocks、并行与串行

Anthropic Messages 的输出单元是 assistant 消息中的 `content[]` 数组。与 Responses 的顶层 `output[]` 不同，Anthropic 把所有输出内容（文本、思考、工具调用、服务端工具调用）都嵌套在一个 `Message.content[]` 里。

同一个“并行读取两个文件”的工具调用，在 Anthropic 中表现为一个 assistant 消息里的多个 `tool_use` block：

```json
{
  "role": "assistant",
  "content": [
    {"type": "tool_use", "id": "tu_1", "name": "read_file", "input": {"path": "src/main.rs"}},
    {"type": "tool_use", "id": "tu_2", "name": "read_file", "input": {"path": "Cargo.toml"}}
  ]
}
```

对应的工具结果在下一个 user 消息中以 `tool_result` block 回填，同样可以包含多个 result block：

```json
{
  "role": "user",
  "content": [
    {"type": "tool_result", "tool_use_id": "tu_1", "content": "fn main() {...}"},
    {"type": "tool_result", "tool_use_id": "tu_2", "content": "[package]\nname = ..."}
  ]
}
```

### 并行控制

Anthropic 通过 `tool_choice` 中的 `disable_parallel_tool_use` 字段控制是否允许并行工具调用：

```json
{"type": "auto", "disable_parallel_tool_use": true}
```

当设为 `true` 时，模型每次只会产出一个 `tool_use` block。省略或设为 `false` 时，模型可以在单个 assistant 消息中产出多个 `tool_use` block。

OpenAI 协议的对应字段是请求级 `parallel_tool_calls: false`。proxai 在 `openai_responses -> anthropic_messages` 转换时，将 `parallel_tool_calls: false` 映射为 `tool_choice.disable_parallel_tool_use: true`。

### 严格邻接约束

Anthropic 要求工具调用和工具结果严格相邻：

1. assistant 消息包含一个或多个 `tool_use` block。
2. 紧接着的 user 消息必须包含对应的 `tool_result` block（每个 `tool_use_id` 恰好匹配一个 result）。
3. 中间不能插入其他 role 的消息。

部分 provider（如 MiniMax M3）严格执行此约束，对拆分或交错的工具轮次会返回 `tool call result does not follow tool call (2013)` 错误。proxai 在翻译时保持相邻 `tool_use` / `tool_result` 的聚合，避免拆散并行工具调用。

### 流式中的并行工具参数

SSE 流式时，多个并行 `tool_use` 的 `input_json_delta` 事件可以交错到达，通过 `ContentBlockStartEvent.index` 和 `ContentBlockDeltaEvent.index` 区分属于哪个 block：

```text
index=0  input_json_delta  {"path":
index=1  input_json_delta  {"path":
index=0  input_json_delta  "src/main.rs"}
index=1  input_json_delta  "Cargo.toml"}
```

这与 Responses 通过 `item_id` + `output_index` 区分并行 item 的方式类似，但 Anthropic 使用的是消息内 content block 的数组 index。

## 完整交互示例

下面用一个简化场景串起客户端、本地工具、服务端工具和 SSE。

用户问：“北京今天适合跑步吗？如果空气质量不好，请查一下官方建议。”

客户端提供一个本地工具 `get_weather`，由客户端自己执行；同时允许上游使用服务端 `web_search`。从协议角度看，`get_weather` 会产生 `tool_use`，结果由客户端下一轮回填；`web_search` 会产生 `server_tool_use` 和对应的 server-tool result block，上游自己完成。

这个示例用于解释协议交互，不是可直接 replay 的测试 fixture。真实上游可能调整具体 server-tool result 内容、事件拆分粒度和 token usage 数值；proxai 当前 Anthropic 路径也不会重建这些事件，只透传上游 bytes。

```text
Client
  |
  | 1. messages + custom get_weather + server web_search + stream=true
  v
proxai
  |
  | 2. 原样转发 anthropic_messages
  v
Anthropic upstream
  |
  | 3. SSE: server_tool_use(web_search) + server result + tool_use(get_weather)
  v
proxai
  |
  | 4. 原始 SSE bytes 透传
  v
Client
  |
  | 5. 本地执行 get_weather
  v
Local tool runtime
  |
  | 6. 下一轮 messages 带 tool_result
  v
proxai -> Anthropic upstream -> proxai -> Client
```

### 第一轮请求

请求里同时声明两类工具：

- `custom` 工具 `get_weather`：本地工具，模型只能请求调用。
- `web_search_20250305`：服务端工具，上游可以自己执行并返回结果。

```json
{
  "model": "claude-sonnet-4-5",
  "max_tokens": 1024,
  "stream": true,
  "system": "你是一个简洁的出行建议助手。",
  "messages": [
    {
      "role": "user",
      "content": "北京今天适合跑步吗？如果空气质量不好，请查一下官方建议。"
    }
  ],
  "tools": [
    {
      "type": "custom",
      "name": "get_weather",
      "description": "查询指定城市的天气和空气质量摘要。",
      "input_schema": {
        "type": "object",
        "properties": {
          "city": { "type": "string" },
          "date": { "type": "string" }
        },
        "required": ["city", "date"]
      }
    },
    {
      "type": "web_search_20250305",
      "name": "web_search",
      "max_uses": 1,
      "allowed_domains": ["www.cma.gov.cn", "www.mee.gov.cn"]
    }
  ],
  "tool_choice": { "type": "auto" }
}
```

对应到本地结构：

- 整个请求是 `MessageCreateParamsBase`。
- `tools[0]` 是 `ToolUnion::Custom(Tool)`。
- `tools[1]` 是 `ToolUnion::WebSearchTool20250305(...)`。
- `tool_choice` 是 `ToolChoice::Auto(...)`。

更完整的结构映射如下。这里是字段映射伪代码，`json!` 表示 `serde_json::json!`，`...` 表示其余 `Option` 字段按需为 `None`：

```rust
MessageCreateParamsBase {
    model: "claude-sonnet-4-5".to_string(),
    max_tokens: 1024,
    stream: Some(true),
    system: Some(SystemPrompt::Text(
        "你是一个简洁的出行建议助手。".to_string(),
    )),
    messages: vec![MessageParam {
        role: Role::User,
        content: MessageParamContent::Text(
            "北京今天适合跑步吗？如果空气质量不好，请查一下官方建议。".to_string(),
        ),
    }],
    tools: Some(vec![
        ToolUnion::Custom(Tool {
            name: "get_weather".to_string(),
            description: Some("查询指定城市的天气和空气质量摘要。".to_string()),
            input_schema: InputSchema {
                type_: "object".to_string(),
                properties: Some(json!({
                    "city": { "type": "string" },
                    "date": { "type": "string" }
                })),
                required: Some(vec!["city".to_string(), "date".to_string()]),
                extra: json!({}),
            },
            type_: Some("custom".to_string()),
            ...
        }),
        ToolUnion::WebSearchTool20250305(WebSearchTool20250305 {
            name: "web_search".to_string(),
            type_: "web_search_20250305".to_string(),
            max_uses: Some(1),
            allowed_domains: Some(vec![
                "www.cma.gov.cn".to_string(),
                "www.mee.gov.cn".to_string(),
            ]),
            ...
        }),
    ]),
    tool_choice: Some(ToolChoice::Auto(ToolChoiceAuto { ... })),
    // 其他可选字段省略为 None。
    ...
}
```

### 第一轮 SSE

服务端工具调用和本地工具调用都出现在同一条 SSE 流中，但语义不同。

```text
event: message_start
data: {
  "type": "message_start",
  "message": {
    "id": "msg_01",
    "type": "message",
    "role": "assistant",
    "content": [],
    "model": "claude-sonnet-4-5",
    "usage": {
      "input_tokens": 120,
      "output_tokens": 1
    }
  }
}

event: content_block_start
data: {
  "type": "content_block_start",
  "index": 0,
  "content_block": {
    "type": "server_tool_use",
    "id": "srvu_01",
    "name": "web_search",
    "caller": { "type": "direct" },
    "input": {
      "query": "北京 空气质量 跑步 官方 建议"
    }
  }
}

event: content_block_stop
data: {
  "type": "content_block_stop",
  "index": 0
}

event: content_block_start
data: {
  "type": "content_block_start",
  "index": 1,
  "content_block": {
    "type": "web_search_tool_result",
    "id": "srvr_01",
    "caller": { "type": "direct" },
    "input": {
      "query": "北京 空气质量 跑步 官方 建议"
    },
    "name": "web_search",
    "content": [
      {
        "type": "web_search_result",
        "title": "官方空气质量与健康建议",
        "url": "https://www.mee.gov.cn/example",
        "encrypted_content": "...",
        "page_age": null
      }
    ],
    "error": null
  }
}

event: content_block_stop
data: {
  "type": "content_block_stop",
  "index": 1
}

event: content_block_start
data: {
  "type": "content_block_start",
  "index": 2,
  "content_block": {
    "type": "tool_use",
    "id": "toolu_01",
    "name": "get_weather",
    "caller": { "type": "direct" },
    "input": {}
  }
}

event: content_block_delta
data: {
  "type": "content_block_delta",
  "index": 2,
  "delta": {
    "type": "input_json_delta",
    "partial_json": "{\"city\":\"北京\""
  }
}

event: content_block_delta
data: {
  "type": "content_block_delta",
  "index": 2,
  "delta": {
    "type": "input_json_delta",
    "partial_json": ",\"date\":\"today\"}"
  }
}

event: content_block_stop
data: {
  "type": "content_block_stop",
  "index": 2
}

event: message_delta
data: {
  "type": "message_delta",
  "delta": {
    "stop_reason": "tool_use",
    "stop_sequence": null
  },
  "usage": {
    "output_tokens": 72,
    "server_tool_use": {
      "web_search_requests": 1,
      "web_fetch_requests": 0
    }
  }
}

event: message_stop
data: {
  "type": "message_stop"
}
```

逐段对应到本地结构：

```rust
MessageStreamEvent::MessageStart(MessageStartEvent {
    message: Message {
        id: "msg_01".to_string(),
        type_: MessageType::Message,
        role: Role::Assistant,
        content: vec![],
        model: "claude-sonnet-4-5".to_string(),
        usage: Usage {
            input_tokens: 120,
            output_tokens: 1,
            ...
        },
        ...
    },
})
```

```rust
MessageStreamEvent::ContentBlockStart(ContentBlockStartEvent {
    index: 0,
    content_block: ContentBlock::ServerToolUse(ServerToolUseBlock {
        id: "srvu_01".to_string(),
        caller: ToolCaller::Direct(DirectCaller),
        name: ServerToolName::WebSearch,
        input: json!({ "query": "北京 空气质量 跑步 官方 建议" }),
    }),
})

MessageStreamEvent::ContentBlockStop(ContentBlockStopEvent { index: 0 })
```

```rust
MessageStreamEvent::ContentBlockStart(ContentBlockStartEvent {
    index: 1,
    content_block: ContentBlock::WebSearchToolResult(ServerToolResultBlock {
        id: "srvr_01".to_string(),
        caller: ToolCaller::Direct(DirectCaller),
        input: json!({ "query": "北京 空气质量 跑步 官方 建议" }),
        name: ServerToolName::WebSearch,
        content: ServerToolResultContent::Data(json!([
            {
                "type": "web_search_result",
                "title": "官方空气质量与健康建议",
                "url": "https://www.mee.gov.cn/example",
                "encrypted_content": "...",
                "page_age": null
            }
        ])),
        type_: "web_search_tool_result".to_string(),
        ...
    }),
})

MessageStreamEvent::ContentBlockStop(ContentBlockStopEvent { index: 1 })
```

```rust
MessageStreamEvent::ContentBlockStart(ContentBlockStartEvent {
    index: 2,
    content_block: ContentBlock::ToolUse(ToolUseBlock {
        id: "toolu_01".to_string(),
        caller: ToolCaller::Direct(DirectCaller),
        name: "get_weather".to_string(),
        input: json!({}),
    }),
})

MessageStreamEvent::ContentBlockDelta(ContentBlockDeltaEvent {
    index: 2,
    delta: ContentBlockDelta::InputJsonDelta(InputJsonDelta {
        partial_json: "{\"city\":\"北京\"".to_string(),
    }),
})

MessageStreamEvent::ContentBlockDelta(ContentBlockDeltaEvent {
    index: 2,
    delta: ContentBlockDelta::InputJsonDelta(InputJsonDelta {
        partial_json: ",\"date\":\"today\"}".to_string(),
    }),
})

MessageStreamEvent::ContentBlockStop(ContentBlockStopEvent { index: 2 })
```

```rust
MessageStreamEvent::MessageDelta(MessageDeltaEvent {
    delta: MessageDelta {
        stop_reason: Some(StopReason::ToolUse),
        ...
    },
    usage: MessageDeltaUsage {
        output_tokens: 72,
        server_tool_use: Some(ServerToolUsage {
            web_search_requests: 1,
            web_fetch_requests: 0,
        }),
        ...
    },
})

MessageStreamEvent::MessageStop(MessageStopEvent)
```

这里的 `MessageDelta` 是 message 级别的状态增量，不是文本内容增量：

- `ContentBlockDelta` 更新某个 `content_block` 的内容，例如 text delta 或 tool input 的 partial JSON。
- `MessageDelta` 更新整条 assistant message 的元数据，例如 `stop_reason`、`stop_sequence`、`container`、`stop_details`。
- `MessageDeltaEvent.usage` 携带到这一刻为止的 token/server-tool usage 增量或汇总。

在这个例子里，`stop_reason: ToolUse` 表示模型已经产出一个本地工具调用，并主动停下来等待客户端回填 `tool_result`。`server_tool_use` 则说明同一轮里上游已经执行过 1 次服务端 web search。

这段流对应的图示：

```text
SSE content blocks

index=0  server_tool_use(web_search)
         id=srvu_01
           |
           v
index=1  web_search_tool_result
         id=srvr_01
         caller=direct

index=2  tool_use(get_weather)
         id=toolu_01
           |
           v
         Client must run local tool
```

关键点：

- `server_tool_use` 由上游服务执行，客户端不需要回填它的结果。
- `web_search_tool_result` 在当前 wire model 中落到 `WebSearchToolResultBlock`，其中 `content` 是 `WebSearchToolResultBlockContent`，可以是错误或搜索结果数组。
- `tool_use.id = toolu_01` 是本地工具调用 ID，客户端必须执行 `get_weather`，并在下一轮用 `tool_result.tool_use_id = toolu_01` 回填。
- `input_json_delta.partial_json` 是增量 JSON 片段，客户端按 `index=2` 聚合得到 `{"city":"北京","date":"today"}`。
- `message_delta.stop_reason = "tool_use"` 表示本轮停在等待客户端工具结果的位置。

这轮流里的 ID 关系可以按下表理解：

| ID | 来源 | 后续怎么用 |
| --- | --- | --- |
| `srvu_01` | `server_tool_use` | 上游服务内部执行，客户端不用回填 |
| `srvr_01` | `web_search_tool_result` | 服务端工具结果块自己的 ID，仅用于描述该结果块 |
| `toolu_01` | `tool_use(get_weather)` | 客户端执行本地工具后，用 `tool_result.tool_use_id` 回填 |

客户端侧聚合 SSE 时至少要维护两张临时表：

```text
content_blocks[index] = started block
partial_tool_inputs[index] += input_json_delta.partial_json
```

当 `content_block_stop(index=2)` 到达时，客户端可以把 `partial_tool_inputs[2]` 解析成 JSON，并用同一个 block 的 `ToolUseBlock.id` 作为本地工具调用 ID。

### 本地工具结果回填

客户端执行本地 `get_weather` 后，下一轮请求把上一轮 assistant 的 `tool_use` 和新的 `tool_result` 一起放进 `messages`。示例省略前一轮的完整文本内容，只保留协议关键块：

```json
{
  "model": "claude-sonnet-4-5",
  "max_tokens": 1024,
  "stream": true,
  "messages": [
    {
      "role": "user",
      "content": "北京今天适合跑步吗？如果空气质量不好，请查一下官方建议。"
    },
    {
      "role": "assistant",
      "content": [
        {
          "type": "tool_use",
          "id": "toolu_01",
          "name": "get_weather",
          "input": { "city": "北京", "date": "today" }
        }
      ]
    },
    {
      "role": "user",
      "content": [
        {
          "type": "tool_result",
          "tool_use_id": "toolu_01",
          "content": "北京今天气温 18-27C，轻度污染，PM2.5 约 85，傍晚有风。",
          "is_error": false
        }
      ]
    }
  ]
}
```

对应到本地结构：

- assistant content 里的 `tool_use` 是 `ContentBlockParam::ToolUse(ToolUseBlockParam)`。
- user content 里的 `tool_result` 是 `ContentBlockParam::ToolResult(ToolResultBlockParam)`。
- `ToolResultBlockParam.tool_use_id` 必须等于上一轮 `ToolUseBlock.id`。

结构映射如下：

```rust
MessageCreateParamsBase {
    model: "claude-sonnet-4-5".to_string(),
    max_tokens: 1024,
    stream: Some(true),
    messages: vec![
        MessageParam {
            role: Role::User,
            content: MessageParamContent::Text(
                "北京今天适合跑步吗？如果空气质量不好，请查一下官方建议。".to_string(),
            ),
        },
        MessageParam {
            role: Role::Assistant,
            content: MessageParamContent::Blocks(vec![
                ContentBlockParam::ToolUse(ToolUseBlockParam {
                    id: "toolu_01".to_string(),
                    name: "get_weather".to_string(),
                    input: json!({ "city": "北京", "date": "today" }),
                    ...
                }),
            ]),
        },
        MessageParam {
            role: Role::User,
            content: MessageParamContent::Blocks(vec![
                ContentBlockParam::ToolResult(ToolResultBlockParam {
                    tool_use_id: "toolu_01".to_string(),
                    content: Some(ToolResultContentParam::Text(
                        "北京今天气温 18-27C，轻度污染，PM2.5 约 85，傍晚有风。"
                            .to_string(),
                    )),
                    is_error: Some(false),
                    ...
                }),
            ]),
        },
    ],
    // 其他可选字段省略为 None。
    ...
}
```

### 第二轮 SSE 最终回答

上游拿到本地工具结果后继续生成最终文本：

```text
event: message_start
data: {
  "type": "message_start",
  "message": {
    "id": "msg_02",
    "type": "message",
    "role": "assistant",
    "content": [],
    "model": "claude-sonnet-4-5",
    "usage": {
      "input_tokens": 210,
      "output_tokens": 1
    }
  }
}

event: content_block_start
data: {
  "type": "content_block_start",
  "index": 0,
  "content_block": {
    "type": "text",
    "text": ""
  }
}

event: content_block_delta
data: {
  "type": "content_block_delta",
  "index": 0,
  "delta": {
    "type": "text_delta",
    "text": "今天北京不太适合高强度户外跑步。"
  }
}

event: content_block_delta
data: {
  "type": "content_block_delta",
  "index": 0,
  "delta": {
    "type": "text_delta",
    "text": "空气质量为轻度污染，建议改为低强度慢跑或室内训练；如果外出，避开交通高峰并缩短时长。"
  }
}

event: content_block_stop
data: {
  "type": "content_block_stop",
  "index": 0
}

event: message_delta
data: {
  "type": "message_delta",
  "delta": {
    "stop_reason": "end_turn",
    "stop_sequence": null
  },
  "usage": {
    "output_tokens": 48
  }
}

event: message_stop
data: {
  "type": "message_stop"
}
```

对应到本地结构：

```rust
MessageStreamEvent::MessageStart(MessageStartEvent {
    message: Message {
        id: "msg_02".to_string(),
        type_: MessageType::Message,
        role: Role::Assistant,
        content: vec![],
        model: "claude-sonnet-4-5".to_string(),
        usage: Usage {
            input_tokens: 210,
            output_tokens: 1,
            ...
        },
        ...
    },
})

MessageStreamEvent::ContentBlockStart(ContentBlockStartEvent {
    index: 0,
    content_block: ContentBlock::Text(TextBlock {
        text: String::new(),
        ...
    }),
})

MessageStreamEvent::ContentBlockDelta(ContentBlockDeltaEvent {
    index: 0,
    delta: ContentBlockDelta::TextDelta(TextDelta {
        text: "今天北京不太适合高强度户外跑步。".to_string(),
    }),
})

MessageStreamEvent::ContentBlockDelta(ContentBlockDeltaEvent {
    index: 0,
    delta: ContentBlockDelta::TextDelta(TextDelta {
        text: "空气质量为轻度污染，建议改为低强度慢跑或室内训练；如果外出，避开交通高峰并缩短时长。"
            .to_string(),
    }),
})

MessageStreamEvent::ContentBlockStop(ContentBlockStopEvent { index: 0 })

MessageStreamEvent::MessageDelta(MessageDeltaEvent {
    delta: MessageDelta {
        stop_reason: Some(StopReason::EndTurn),
        ...
    },
    usage: MessageDeltaUsage {
        output_tokens: 48,
        ...
    },
})

MessageStreamEvent::MessageStop(MessageStopEvent)
```

这里的 `TextDelta.text` 是增量片段，不是到当前为止的完整文本。客户端需要按 `index` 找到对应 content block，然后顺序追加：

```text
text_blocks[0] = ""
text_blocks[0] += "今天北京不太适合高强度户外跑步。"
text_blocks[0] += "空气质量为轻度污染，建议改为低强度慢跑或室内训练；如果外出，避开交通高峰并缩短时长。"
```

当 `ContentBlockStopEvent { index: 0 }` 到达时，`text_blocks[0]` 才是这个文本块的完整内容。

最终回答的流式图示：

```text
message_start
  |
  v
content_block_start(index=0, text)
  |
  +-- text_delta: 今天北京不太适合...
  |
  +-- text_delta: 空气质量为轻度污染...
  |
  v
content_block_stop(index=0)
  |
  v
message_delta(stop_reason=end_turn)
  |
  v
message_stop
```

proxai 在这条链路中不重组这些事件。当前 Anthropic provider 默认保留原始 SSE bytes；`AnthropicSseObserver` 使用 `SseEventScanner` 扫描事件并归纳到 `AnthropicResponseState`，用于 completed/closed/error 日志。对 `AnthropicCompatible` provider，响应层会做 provider-local SSE payload normalization 后再输出。

### 示例边界

这个例子刻意保留了三个边界：

- 请求头没有展开。真实 Anthropic 请求还需要 provider 侧认证和版本相关 headers；proxai 的 provider/auth 层负责把配置里的 key 转成上游需要的认证头。
- 服务端工具结果是示意性的。文档重点是 `server_tool_use` 由上游执行、本地 `tool_use` 由客户端执行，而不是锁定某个 provider 的完整 web result JSON。
- 当前 proxai 不把 Anthropic SSE 重组为新的语义流，也不对 Anthropic 工具参数流做超时注入；`MessageStreamEvent` 用于 provider response 观察、摘要和跨协议转换。

## proxai 当前处理方式

`anthropic_messages -> anthropic_messages` 当前是透传：

1. ingress 校验 JSON 和非空 `model`。
2. translation 构造 `ProviderRequest::AnthropicMessages`，保留原始 body。
3. provider 直接转发到上游。
4. 非流式响应原样返回并可 capture。
5. SSE 响应保留原始 bytes 和 headers；`AnthropicSseObserver` 使用 `SseEventScanner` 扫描事件，将 `MessageStreamEvent` 归纳到 `AnthropicResponseState`，并记录 completed/closed/error 日志。`AnthropicCompatible` provider 会先经过 provider-local SSE normalization。

当前 Anthropic observer 不重组语义流，也不注入工具参数超时诊断。协议层的 `MessageStreamEvent` 主要用于 provider response 观察、摘要和跨协议翻译。
