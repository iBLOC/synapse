# BaseChatModel

LLM provider abstraction.

## Definition

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

## Options

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

## Implementations

- `OpenAIModel` — OpenAI API
- `OpenAIChatModel` — Lower-level OpenAI
- `AnthropicClient` — Anthropic Claude

## See Also

- [Model concepts](../concepts/model.md)
