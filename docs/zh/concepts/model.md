---
title: Model Providers
---

# 模型 Provider

Synapse 通过 `LLMProvider` trait 为 LLM provider 提供统一接口。

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

## 内置 Provider

| 提供方 | 实现类 | 环境变量 |
|----------|-------|---------|
| OpenAI | `OpenAIProvider` | `OPENAI_API_KEY` |
| Anthropic | `AnthropicProvider` | `ANTHROPIC_API_KEY` |
| Azure OpenAI | `AzureProvider` | `AZURE_OPENAI_API_KEY` |

## 快速开始

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

## Provider 注册表

用中央注册表管理多个 provider：

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

### 从配置构建

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

## 自定义 Provider

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

## 通过环境变量配置

| 变量 | 提供方 | 说明 |
|----------|----------|-------------|
| `OPENAI_API_KEY` | OpenAI | API 密钥 |
| `OPENAI_API_BASE` | OpenAI | 自定义 base URL（用于 Ollama 等） |
| `OPENAI_MODEL` | OpenAI | 默认 model |
| `ANTHROPIC_API_KEY` | Anthropic | API 密钥 |
| `ANTHROPIC_BASE_URL` | Anthropic | 自定义 base URL |
| `ANTHROPIC_MODEL` | Anthropic | 默认 model |

## 相关阅读
