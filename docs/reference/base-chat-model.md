---
title: BaseChatModel
---

# BaseChatModel · 基础 Chat Model

LLM provider abstraction.
*LLM provider 抽象。*

## Definition · 定义

```rust
#[async_trait]
pub trait BaseChatModel: Send + Sync {
    async fn generate(
        &self,
        input: Vec<Message>,
        options: GenerateOptions,
    ) -> ModelResult<Message>;

    async fn stream(
        &self,
        input: Vec<Message>,
        options: GenerateOptions,
    ) -> ModelResult<StreamReader>;
}
```

## Options · 选项

```rust
pub struct GenerateOptions {
    pub model: Option<String>,
    pub temperature: Option<f32>,
    pub max_tokens: Option<u32>,
    pub top_p: Option<f32>,
    pub tools: Option<Vec<ToolInfo>>,
    pub tool_choice: Option<ToolChoice>,
}

pub enum ToolChoice {
    Auto,
    None,
    Function { name: String },
}

pub enum ModelError {
    Api(String),
    RateLimit(String),
    InvalidRequest(String),
    ContextLengthExceeded,
    Timeout,
    StreamError(String),
}
```

## Implementations · 实现

- `OpenAIModel` — OpenAI API
  *`OpenAIModel` —— OpenAI API*
- `OpenAIChatModel` — Lower-level OpenAI
  *`OpenAIChatModel` —— 更底层的 OpenAI*
- `AnthropicClient` — Anthropic Claude
  *`AnthropicClient` —— Anthropic Claude*

## See Also · 相关阅读

- [Model concepts](/concepts/model)
