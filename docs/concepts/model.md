---
title: Model
---

# Model

`BaseChatModel` is the abstraction over LLM providers.

## Trait Definition

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

## Built-in Implementations

| Model | Notes |
|-------|-------|
| `OpenAIModel` | OpenAI API (GPT-4, GPT-3.5) |
| `OpenAIChatModel` | Lower-level OpenAI interface |
| `AnthropicClient` | Anthropic API (Claude) |
| `OpenAIEmbedding` | For embeddings (used in RAG) |

All built-in models use the cross-platform `HttpClient` trait.

## GenerateOptions

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
```

## Example: OpenAI

```rust
use std::sync::Arc;
use synapse::components::model::OpenAIModel;

let model = Arc::new(OpenAIModel::new("gpt-4", "sk-..."));
```

## Example: Anthropic

```rust
use std::sync::Arc;
use synapse::llm::anthropic::AnthropicClient;

let model = Arc::new(AnthropicClient::new("claude-3-5-sonnet-20241022", "sk-ant-..."));
```

## Custom Model

```rust
use async_trait::async_trait;
use synapse::components::model::{BaseChatModel, GenerateOptions, ModelError, ModelResult};
use synapse::schema::Message;

struct MyLocalModel;

#[async_trait]
impl BaseChatModel for MyLocalModel {
    async fn generate(
        &self,
        _input: Vec<Message>,
        _options: GenerateOptions,
    ) -> ModelResult<Message> {
        Ok(Message::assistant("Hello from local LLM"))
    }
    async fn stream(
        &self,
        _input: Vec<Message>,
        _options: GenerateOptions,
    ) -> ModelResult<synapse::components::model::StreamReader> {
        Err(ModelError::Api("not implemented".into()))
    }
}
```
