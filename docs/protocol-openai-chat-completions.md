# OpenAI Chat Completions 协议

proxai 中的协议名是 `openai_chat_completions`，主要类型位于：

- `src/protocol/openai/chat_completions/wire`
- `src/protocol/openai/chat_completions/request`
- `src/protocol/openai/chat_completions/response`
- `src/provider/openai/chat_completions`

## 请求模型

Chat Completions 的核心请求是一个有序 `messages` 列表加一组生成参数。proxai 的轻量投影是 `RequestProjection`：

```rust
struct RequestProjection {
    model: Option<String>,
    stream: Option<bool>,
    stream_options: Option<ChatCompletionStreamOptions>,
    tools: Option<Vec<ChatCompletionTools>>,
    tool_choice: Option<ChatCompletionToolChoiceOption>,
    parallel_tool_calls: Option<bool>,
    response_format: Option<ResponseFormat>,
    reasoning_effort: Option<ReasoningEffort>,
    max_completion_tokens: Option<u32>,
    temperature: Option<f32>,
    top_p: Option<f32>,
    ...
}
```

和 Responses projection 一样，Chat projection 刻意不保留完整 `messages`，避免日志和路由提示依赖私有 prompt 内容。完整转发 body 仍来自 normalized payload。

请求消息由 `ChatCompletionRequestMessage` 表示：

```rust
enum ChatCompletionRequestMessage {
    Developer(ChatCompletionRequestDeveloperMessage),
    System(ChatCompletionRequestSystemMessage),
    User(ChatCompletionRequestUserMessage),
    Assistant(ChatCompletionRequestAssistantMessage),
    Tool(ChatCompletionRequestToolMessage),
    Function(ChatCompletionRequestFunctionMessage),
}
```

不同角色允许的内容不同：

- `system` / `developer`：文本或 text part。
- `user`：文本、图片、音频、文件等 content part。
- `assistant`：文本、拒绝内容、音频引用、`tool_calls`。
- `tool`：工具结果，通过 `tool_call_id` 关联。
- `function`：旧版函数消息，保留兼容但不是新路径重点。

## 非流式响应

非流式响应是 `CreateChatCompletionResponse`：

```rust
struct CreateChatCompletionResponse {
    id: String,
    choices: Vec<ChatChoice>,
    created: u32,
    model: String,
    object: String,
    usage: Option<CompletionUsage>,
    service_tier: Option<ServiceTier>,
}
```

每个 `ChatChoice` 包含一个 assistant message：

```rust
struct ChatChoice {
    index: u32,
    message: ChatCompletionResponseMessage,
    finish_reason: Option<FinishReason>,
    logprobs: Option<ChatChoiceLogprobs>,
}
```

`finish_reason` 支持：

- `stop`
- `length`
- `tool_calls`
- `content_filter`
- `function_call`

当模型需要调用工具时，通常 `finish_reason = tool_calls`，并在 message 内返回 `tool_calls`。

## 用户端工具调用

Chat Completions 的主要工具模型是用户端工具调用：客户端声明工具，模型返回调用请求，客户端执行后把结果作为下一轮 `tool` message 传回。

工具定义：

```rust
enum ChatCompletionTools {
    Function(ChatCompletionTool),
    Custom(CustomToolChatCompletions),
}

struct FunctionObject {
    name: String,
    description: Option<String>,
    parameters: Option<Value>,
    strict: Option<bool>,
}
```

工具选择：

```rust
enum ChatCompletionToolChoiceOption {
    AllowedTools(ChatCompletionAllowedToolsChoice),
    Function(ChatCompletionNamedToolChoice),
    Custom(ChatCompletionNamedToolChoiceCustom),
    Mode(ToolChoiceOptions),
}
```

模型返回的工具调用：

```rust
enum ChatCompletionMessageToolCalls {
    Function(ChatCompletionMessageToolCall),
    Custom(ChatCompletionMessageCustomToolCall),
}

struct ChatCompletionMessageToolCall {
    id: String,
    function: FunctionCall,
}

struct FunctionCall {
    name: String,
    arguments: String,
}
```

客户端执行后用 `tool` message 回填：

```rust
struct ChatCompletionRequestToolMessage {
    content: ChatCompletionRequestToolMessageContent,
    tool_call_id: String,
}
```

关联字段是 `tool_call_id`，它必须对应模型返回的 `ChatCompletionMessageToolCall.id`。

## 服务端工具调用

Chat Completions 的 proxai wire model 没有像 Responses 那样的 hosted tool item 生命周期，也没有 Anthropic 那样的 `server_tool_use` block。

当前结构里和服务端能力更接近的是请求级选项，例如 `web_search_options`、`response_format`、`audio`、`service_tier` 等。它们影响上游服务行为，但不会在 Chat Completions 协议中形成独立的 server-tool-use/result block。

因此在 proxai 内部，Chat Completions 路径的工具重点是：

- `tools`：客户端可执行工具定义。
- `tool_choice`：模型是否/如何选择工具。
- assistant `tool_calls`：模型请求客户端执行工具。
- 后续 `tool` messages：客户端回传工具结果。

## SSE 流式

Chat Completions 流式响应是一串 `CreateChatCompletionStreamResponse` chunk：

```rust
struct CreateChatCompletionStreamResponse {
    id: String,
    choices: Vec<ChatChoiceStream>,
    created: u32,
    model: String,
    object: String,
    usage: Option<CompletionUsage>,
    service_tier: Option<ServiceTier>,
}
```

每个 choice 的增量在 `delta` 里：

```rust
struct ChatCompletionStreamResponseDelta {
    content: Option<String>,
    tool_calls: Option<Vec<ChatCompletionMessageToolCallChunk>>,
    role: Option<Role>,
    refusal: Option<String>,
}
```

工具调用参数也是增量字符串：

```rust
struct ChatCompletionMessageToolCallChunk {
    index: u32,
    id: Option<String>,
    type: Option<FunctionType>,
    function: Option<FunctionCallStream>,
}

struct FunctionCallStream {
    name: Option<String>,
    arguments: Option<String>,
}
```

客户端需要按 `choice.index` 和 `tool_call_chunk.index` 聚合工具参数，直到该 choice 出现 `finish_reason = tool_calls` 或其他终止原因。

proxai 的 Chat Completions provider 使用通用 stream mechanics 保留 SSE bytes，并由 Chat observer 解析 `chat.completion.chunk`，用于 usage、finish reason 和日志摘要。它不像 Responses observer 那样注入工具参数超时诊断。

## 完整交互示例

下面用同一个跑步建议场景展示 Chat Completions 的工具调用过程。

用户问：“北京今天适合跑步吗？如果空气质量不好，请参考公开信息给建议。”

Chat Completions 的工具调用核心是本地工具：模型返回 `tool_calls`，客户端执行工具后用下一轮 `tool` message 回填。这个协议没有 Responses/Anthropic 那样的服务端工具 item/block 生命周期；`web_search_options` 这类能力是请求级选项，只影响上游服务行为，不会在 SSE 中形成独立的 server-tool-use/result 事件。

```text
Client
  |
  | 1. messages + tools(function get_weather) + web_search_options + stream=true
  v
proxai
  |
  | 2. 原样转发 openai_chat_completions
  v
OpenAI-compatible upstream
  |
  | 3. SSE: delta.tool_calls(function arguments)
  v
proxai
  |
  | 4. 原始 SSE bytes 透传，Chat observer 记录 finish_reason/usage
  v
Client
  |
  | 5. 本地执行 get_weather
  v
Local tool runtime
  |
  | 6. 下一轮 messages 带 tool role 结果
  v
proxai -> upstream -> proxai -> Client
```

### 第一轮请求

```json
{
  "model": "gpt-5.4",
  "stream": true,
  "messages": [
    {
      "role": "system",
      "content": "你是一个简洁的出行建议助手。"
    },
    {
      "role": "user",
      "content": "北京今天适合跑步吗？如果空气质量不好，请参考公开信息给建议。"
    }
  ],
  "tools": [
    {
      "type": "function",
      "function": {
        "name": "get_weather",
        "description": "查询指定城市的天气和空气质量摘要。",
        "parameters": {
          "type": "object",
          "properties": {
            "city": { "type": "string" },
            "date": { "type": "string" }
          },
          "required": ["city", "date"]
        },
        "strict": true
      }
    }
  ],
  "tool_choice": "auto",
  "parallel_tool_calls": false,
  "web_search_options": {
    "search_context_size": "low",
    "user_location": {
      "type": "approximate",
      "approximate": {
        "city": "Beijing",
        "country": "CN",
        "timezone": "Asia/Shanghai"
      }
    }
  }
}
```

对应结构映射。这里是字段映射伪代码，`json!` 表示 `serde_json::json!`，`...` 表示其余可选字段省略：

```rust
CreateChatCompletionRequest {
    model: "gpt-5.4".to_string(),
    stream: Some(true),
    messages: vec![
        ChatCompletionRequestMessage::System(ChatCompletionRequestSystemMessage {
            content: ChatCompletionRequestSystemMessageContent::Text(
                "你是一个简洁的出行建议助手。".to_string(),
            ),
            ...
        }),
        ChatCompletionRequestMessage::User(ChatCompletionRequestUserMessage {
            content: ChatCompletionRequestUserMessageContent::Text(
                "北京今天适合跑步吗？如果空气质量不好，请参考公开信息给建议。".to_string(),
            ),
            ...
        }),
    ],
    tools: Some(vec![
        ChatCompletionTools::Function(ChatCompletionTool {
            function: FunctionObject {
                name: "get_weather".to_string(),
                description: Some("查询指定城市的天气和空气质量摘要。".to_string()),
                parameters: Some(json!({
                    "type": "object",
                    "properties": {
                        "city": { "type": "string" },
                        "date": { "type": "string" }
                    },
                    "required": ["city", "date"]
                })),
                strict: Some(true),
            },
        }),
    ]),
    tool_choice: Some(ChatCompletionToolChoiceOption::Mode(ToolChoiceOptions::Auto)),
    parallel_tool_calls: Some(false),
    web_search_options: Some(WebSearchOptions {
        search_context_size: Some(WebSearchContextSize::Low),
        user_location: Some(WebSearchUserLocation {
            r#type: WebSearchUserLocationType::Approximate,
            approximate: WebSearchLocation {
                city: Some("Beijing".to_string()),
                country: Some("CN".to_string()),
                timezone: Some("Asia/Shanghai".to_string()),
                ...
            },
        }),
    }),
    ...
}
```

### 第一轮 SSE

Chat Completions 流式响应里，工具调用参数在 `choices[].delta.tool_calls[].function.arguments` 中增量到达。

```text
data: {
  "id": "chatcmpl_01",
  "object": "chat.completion.chunk",
  "created": 1770000000,
  "model": "gpt-5.4",
  "choices": [
    {
      "index": 0,
      "delta": { "role": "assistant" },
      "finish_reason": null
    }
  ]
}

data: {
  "id": "chatcmpl_01",
  "object": "chat.completion.chunk",
  "created": 1770000000,
  "model": "gpt-5.4",
  "choices": [
    {
      "index": 0,
      "delta": {
        "tool_calls": [
          {
            "index": 0,
            "id": "call_weather_01",
            "type": "function",
            "function": {
              "name": "get_weather",
              "arguments": "{\"city\":\"北京\""
            }
          }
        ]
      },
      "finish_reason": null
    }
  ]
}

data: {
  "id": "chatcmpl_01",
  "object": "chat.completion.chunk",
  "created": 1770000000,
  "model": "gpt-5.4",
  "choices": [
    {
      "index": 0,
      "delta": {
        "tool_calls": [
          {
            "index": 0,
            "function": {
              "arguments": ",\"date\":\"today\"}"
            }
          }
        ]
      },
      "finish_reason": null
    }
  ]
}

data: {
  "id": "chatcmpl_01",
  "object": "chat.completion.chunk",
  "created": 1770000000,
  "model": "gpt-5.4",
  "choices": [
    {
      "index": 0,
      "delta": {},
      "finish_reason": "tool_calls"
    }
  ],
  "usage": {
    "prompt_tokens": 140,
    "completion_tokens": 24,
    "total_tokens": 164
  }
}

data: [DONE]
```

对应结构映射：

```rust
CreateChatCompletionStreamResponse {
    id: "chatcmpl_01".to_string(),
    object: "chat.completion.chunk".to_string(),
    created: 1770000000,
    model: "gpt-5.4".to_string(),
    choices: vec![ChatChoiceStream {
        index: 0,
        delta: ChatCompletionStreamResponseDelta {
            role: Some(Role::Assistant),
            ...
        },
        ...
    }],
    ...
}
```

```rust
CreateChatCompletionStreamResponse {
    choices: vec![ChatChoiceStream {
        index: 0,
        delta: ChatCompletionStreamResponseDelta {
            tool_calls: Some(vec![
                ChatCompletionMessageToolCallChunk {
                    index: 0,
                    id: Some("call_weather_01".to_string()),
                    r#type: Some(FunctionType::Function),
                    function: Some(FunctionCallStream {
                        name: Some("get_weather".to_string()),
                        arguments: Some("{\"city\":\"北京\"".to_string()),
                    }),
                },
            ]),
            ...
        },
        ...
    }],
    ...
}

CreateChatCompletionStreamResponse {
    choices: vec![ChatChoiceStream {
        index: 0,
        delta: ChatCompletionStreamResponseDelta {
            tool_calls: Some(vec![
                ChatCompletionMessageToolCallChunk {
                    index: 0,
                    function: Some(FunctionCallStream {
                        arguments: Some(",\"date\":\"today\"}".to_string()),
                        ...
                    }),
                    ...
                },
            ]),
            ...
        },
        ...
    }],
    ...
}
```

```rust
CreateChatCompletionStreamResponse {
    choices: vec![ChatChoiceStream {
        index: 0,
        delta: ChatCompletionStreamResponseDelta { ... },
        finish_reason: Some(FinishReason::ToolCalls),
        ...
    }],
    usage: Some(CompletionUsage { ... }),
    ...
}
```

客户端聚合规则：

```text
tool_calls[(choice.index=0, tool.index=0)].id = "call_weather_01"
tool_calls[(0, 0)].name = "get_weather"
tool_calls[(0, 0)].arguments += "{\"city\":\"北京\""
tool_calls[(0, 0)].arguments += ",\"date\":\"today\"}"
```

当 `finish_reason = "tool_calls"` 到达时，`arguments` 才可以作为完整 JSON 解析。

### 本地工具结果回填

客户端执行本地 `get_weather` 后，下一轮请求需要带上 assistant 的 `tool_calls` 和对应的 `tool` message。

```json
{
  "model": "gpt-5.4",
  "stream": true,
  "messages": [
    {
      "role": "user",
      "content": "北京今天适合跑步吗？如果空气质量不好，请参考公开信息给建议。"
    },
    {
      "role": "assistant",
      "tool_calls": [
        {
          "id": "call_weather_01",
          "type": "function",
          "function": {
            "name": "get_weather",
            "arguments": "{\"city\":\"北京\",\"date\":\"today\"}"
          }
        }
      ]
    },
    {
      "role": "tool",
      "tool_call_id": "call_weather_01",
      "content": "北京今天气温 18-27C，轻度污染，PM2.5 约 85，傍晚有风。"
    }
  ]
}
```

结构映射：

```rust
CreateChatCompletionRequest {
    model: "gpt-5.4".to_string(),
    stream: Some(true),
    messages: vec![
        ChatCompletionRequestMessage::User(ChatCompletionRequestUserMessage {
            content: ChatCompletionRequestUserMessageContent::Text(
                "北京今天适合跑步吗？如果空气质量不好，请参考公开信息给建议。".to_string(),
            ),
            ...
        }),
        ChatCompletionRequestMessage::Assistant(ChatCompletionRequestAssistantMessage {
            tool_calls: Some(vec![
                ChatCompletionMessageToolCalls::Function(ChatCompletionMessageToolCall {
                    id: "call_weather_01".to_string(),
                    function: FunctionCall {
                        name: "get_weather".to_string(),
                        arguments: "{\"city\":\"北京\",\"date\":\"today\"}".to_string(),
                    },
                }),
            ]),
            ...
        }),
        ChatCompletionRequestMessage::Tool(ChatCompletionRequestToolMessage {
            tool_call_id: "call_weather_01".to_string(),
            content: ChatCompletionRequestToolMessageContent::Text(
                "北京今天气温 18-27C，轻度污染，PM2.5 约 85，傍晚有风。".to_string(),
            ),
        }),
    ],
    ...
}
```

### 第二轮 SSE 最终回答

```text
data: {
  "id": "chatcmpl_02",
  "object": "chat.completion.chunk",
  "created": 1770000010,
  "model": "gpt-5.4",
  "choices": [
    {
      "index": 0,
      "delta": {
        "role": "assistant",
        "content": "今天北京不太适合高强度户外跑步。"
      },
      "finish_reason": null
    }
  ]
}

data: {
  "id": "chatcmpl_02",
  "object": "chat.completion.chunk",
  "created": 1770000010,
  "model": "gpt-5.4",
  "choices": [
    {
      "index": 0,
      "delta": {
        "content": "空气质量为轻度污染，建议改为低强度慢跑或室内训练。"
      },
      "finish_reason": null
    }
  ]
}

data: {
  "id": "chatcmpl_02",
  "object": "chat.completion.chunk",
  "created": 1770000010,
  "model": "gpt-5.4",
  "choices": [
    {
      "index": 0,
      "delta": {},
      "finish_reason": "stop"
    }
  ]
}

data: [DONE]
```

结构映射和聚合规则：

```rust
CreateChatCompletionStreamResponse {
    choices: vec![ChatChoiceStream {
        index: 0,
        delta: ChatCompletionStreamResponseDelta {
            role: Some(Role::Assistant),
            content: Some("今天北京不太适合高强度户外跑步。".to_string()),
            ...
        },
        ...
    }],
    ...
}

CreateChatCompletionStreamResponse {
    choices: vec![ChatChoiceStream {
        index: 0,
        delta: ChatCompletionStreamResponseDelta {
            content: Some("空气质量为轻度污染，建议改为低强度慢跑或室内训练。".to_string()),
            ...
        },
        ...
    }],
    ...
}
```

```text
assistant_text[0] = ""
assistant_text[0] += "今天北京不太适合高强度户外跑步。"
assistant_text[0] += "空气质量为轻度污染，建议改为低强度慢跑或室内训练。"
```

`delta.content` 是新增文本片段，不是完整文本。`finish_reason = "stop"` 表示该 choice 的最终回答结束。

## proxai 当前处理方式

`openai_chat_completions -> openai_chat_completions` 是已接入路径：

1. ingress 用 `async-openai` typed parse 校验请求并提取 model。
2. request preparation 替换上游 model 等转发字段。
3. provider 转发到 `/v1/chat/completions`。
4. 非流式响应解析摘要。
5. SSE 响应保持透传，同时观察 chunk、usage 和 finish reason。
