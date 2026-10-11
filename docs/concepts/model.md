---
title: Model Providers
---

# Model Providers

Synapse provides a unified interface for LLM providers via the `LLMProvider` trait.

## LLMProvider Trait · LLM Provider trait

```rust
#[async_trait]
pub trait LLMProvider: Send + Sync {
    fn provider_name(&self) -> &str;
    fn model(&self) -> &str;

    async fn chat(&self, messages: Vec<Message>, options: ChatOptions) -> LLMResult<Message>;
    async fn stream_chat(&self, messages: Vec<Message>, options: ChatOptions) -> LLMResult<LLMStream>;
    async fn embeddings(&self, texts: Vec<String>) -> LLMResult<Vec<Vec<f32>>>;
}
```

## Built-in Providers

| Provider | Class | Env Var |
|----------|-------|---------|
| OpenAI | `OpenAIProvider` | `OPENAI_API_KEY` |
| Anthropic | `AnthropicProvider` | `ANTHROPIC_API_KEY` |
| Azure OpenAI | `AzureProvider` | `AZURE_OPENAI_API_KEY` |

## Quick Start

```rust
use synapse::providers::openai::OpenAIProvider;
use synapse::providers::{ChatOptions, LLMProvider, Message, Role};

let provider = OpenAIProvider::new("sk-...".to_string());

let messages = vec![
    Message { role: Role::System, content: Content::text("You are helpful.") },
    Message { role: Role::User, content: Content::text("Hello!") },
];

let response = provider.chat(messages, ChatOptions::default()).await?;
println!("{}", response.content.text.unwrap_or_default());
```

## ProviderRegistry

Manage multiple providers with a central registry:

```rust
use synapse::providers::ProviderRegistry;

// From environment variables (OPENAI_API_KEY, ANTHROPIC_API_KEY)
let registry = ProviderRegistry::from_env()?;

// Get default provider
let provider = registry.get_default().unwrap();
let response = provider.chat(messages, options).await?;

// Get specific provider
let anthropic = registry.get("anthropic").unwrap();
```

### From Configuration

```rust
use synapse::providers::{ProviderConfig, ProviderRegistry};

let configs = vec![
    ProviderConfig {
        provider: "openai".to_string(),
        api_key_env: Some("OPENAI_API_KEY".to_string()),
        base_url: None,
        model: Some("gpt-4o".to_string()),
    },
    ProviderConfig {
        provider: "anthropic".to_string,
        api_key: Some("sk-ant-...".to_string()),
        base_url: None,
        model: Some("claude-3-5-sonnet-20241022".to_string()),
    },
];

let registry = ProviderRegistry::from_config(configs)?;
```

## Custom Provider

```rust
use async_trait::async_trait;
use synapse::providers::{ChatOptions, Content, LLMError, LLMProvider, LLMResult, Message};

struct MyLocalModel;

#[async_trait]
impl LLMProvider for MyLocalModel {
    fn provider_name(&self) -> &str { "local" }
    fn model(&self) -> &str { "my-model" }

    async fn chat(&self, messages: Vec<Message>, options: ChatOptions) -> LLMResult<Message> {
        Ok(Message {
            role: Role::Assistant,
            content: Content::text("Hello from local model"),
        })
    }

    async fn stream_chat(&self, _messages: Vec<Message>, _options: ChatOptions) -> LLMResult<LLMStream> {
        Err(LLMError::StreamError("not implemented".into()))
    }

    async fn embeddings(&self, _texts: Vec<String>) -> LLMResult<Vec<Vec<f32>>> {
        Err(LLMError::Api("not implemented".into()))
    }
}
```

## Configuration via Environment Variables

| Variable | Provider | Description |
|----------|----------|-------------|
| `OPENAI_API_KEY` | OpenAI | API key |
| `OPENAI_API_BASE` | OpenAI | Custom base URL (for Ollama, etc.) |
| `OPENAI_MODEL` | OpenAI | Default model |
| `ANTHROPIC_API_KEY` | Anthropic | API key |
| `ANTHROPIC_BASE_URL` | Anthropic | Custom base URL |
| `ANTHROPIC_MODEL` | Anthropic | Default model |

## See Also

- [Agent Trait](/reference/agent-trait) — Using providers in agents
- [RAG Pipeline](/howto/rag-pipeline) — Using embeddings
