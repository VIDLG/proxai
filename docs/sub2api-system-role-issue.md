# OpenAI OAuth 路径未正确处理 Responses `input_text` 形式的 system message

## 背景

版本：`v0.1.121`

使用 OpenAI OAuth 账号时，sub2api 会把请求转发到 ChatGPT internal Codex endpoint：

```text
https://chatgpt.com/backend-api/codex/responses
```

这个上游不接受 `input` 数组中的 `role:"system"` message。实际请求中如果保留 system message，上游会返回类似错误：

```json
{
  "detail": "System messages are not allowed"
}
```

## 复现请求形状

Zed 走 OpenAI Compatible Responses API 时，会发送类似下面的 body：

```json
{
  "model": "gpt-5.5",
  "stream": true,
  "input": [
    {
      "type": "message",
      "role": "system",
      "content": [
        {
          "type": "input_text",
          "text": "You are a coding agent."
        }
      ]
    },
    {
      "type": "message",
      "role": "user",
      "content": [
        {
          "type": "input_text",
          "text": "Hello"
        }
      ]
    }
  ]
}
```

## 当前行为

源码里已经有 `extractSystemMessagesFromInput`，设计上会把 `input` 里的 `role:"system"` 提取到顶层 `instructions`，因为 OAuth 上游不支持 system role。

但当前 `extractTextFromContent` 只识别 content part：

```json
{"type":"text","text":"..."}
```

没有识别 Responses API 常见的：

```json
{"type":"input_text","text":"..."}
```

因此 Zed 发来的 system message 文本提取失败，system message 仍然留在 `input` 中，最终被转发到 ChatGPT internal Codex endpoint 后触发：

```json
{"detail":"System messages are not allowed"}
```

## 期望行为

OpenAI OAuth Codex transform 应该把下面这种 system message：

```json
{
  "type": "message",
  "role": "system",
  "content": [{"type": "input_text", "text": "You are a coding agent."}]
}
```

转换为顶层：

```json
{
  "instructions": "You are a coding agent."
}
```

并从 `input` 中移除该 system message。

如果原请求已经有 `instructions`，建议保持当前逻辑：把 system 提取内容 prepend 到已有 `instructions` 前面。

## 可能修复点

建议在 `backend/internal/service/openai_codex_transform.go` 中更新 `extractTextFromContent`：

- 支持 `type:"input_text"`
- 可继续保留对 `type:"text"` 的支持
- 可考虑顺手支持 `type:"output_text"`，与其他 compat 代码保持一致

并补一个回归测试，覆盖：

- `role:"system"`
- `content:[{"type":"input_text","text":"..."}]`
- transform 后 `instructions` 包含 system 文本
- transform 后 `input` 中不再包含 system message

## `openai_passthrough` 相关风险

另一个相关点是 `openai_passthrough`。

当前 `Forward` 中如果 `account.IsOpenAIPassthroughEnabled()` 为 true，会较早进入 passthrough 分支。这个路径强调“仅替换认证/尽量透传”，可能绕过完整的 `applyCodexOAuthTransform`。

对于 OpenAI OAuth 账号来说，即使是 passthrough，最终上游仍然是 ChatGPT internal Codex endpoint，而它同样不接受 `input` 中的 system message。因此建议 passthrough OAuth 路径也做同样的最小 normalize：

- 将 `input` 中的 `role:"system"` 提取到 `instructions`
- 从 `input` 移除 system message
- 至少支持 `content` part 的 `type:"input_text"`

否则开启 `openai_passthrough` 时，即使普通 OAuth transform 路径修复了，passthrough 仍可能继续触发 `System messages are not allowed`。

## 影响

这会影响使用 Zed 等 OpenAI Compatible Responses API 客户端，并通过 sub2api OpenAI OAuth 账号访问 Codex/Responses 模型的场景。

从实际对比看：

- 原始 `system` message：上游报 `System messages are not allowed`
- 将 system 文本移动到 `instructions`：请求可被上游接受
- 简单把 `system` 改成 `user` 也能绕过错误，但语义不如 `system -> instructions` 正确
