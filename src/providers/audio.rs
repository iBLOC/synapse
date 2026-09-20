//! Audio Provider — 统一 ASR / TTS HTTP 调用抽象。
//!
//! 替代各 Agent 中直接 POST `/v1/audio/{transcriptions,speech}` 的样板代码。
//! 所有 Agent 应使用 `AsrProvider` / `TtsProvider` 而非直接 reqwest 调用。
//!
//! ```ignore
//! use synapse::providers::audio::{AsrProvider, TtsProvider};
//!
//! let asr = AsrProvider::for_endpoint("http://localhost:60080".into(), "whisper-1".into());
//! let text = asr.transcribe(audio_bytes, "wav", Some("zh")).await?;
//!
//! let tts = TtsProvider::for_endpoint("http://localhost:60080".into(), "tts-1".into(), "alloy".into());
//! let mp3 = tts.synthesize("你好", "mp3").await?;
//! ```

use reqwest::Client;
use std::time::Duration;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum AudioError {
    #[error("request error: {0}")]
    Request(String),
    #[error("response error: {0}")]
    Response(String),
    #[error("multipart error: {0}")]
    Multipart(String),
    #[error("status {0}: {1}")]
    Status(u16, String),
}

/// ASR Provider — 语音转文本。
///
/// OpenAI 兼容 `/v1/audio/transcriptions` 接口（multipart form-data）。
pub struct AsrProvider {
    client: Client,
    base_url: String,
    default_model: String,
    default_language: String,
}

impl AsrProvider {
    /// 针对内部 OpenAI 兼容 endpoint 的便捷构造（无 API key）。
    pub fn for_endpoint(
        base_url: impl Into<String>,
        model: impl Into<String>,
    ) -> Self {
        Self {
            client: Client::builder()
                .timeout(Duration::from_secs(60))
                .build()
                .unwrap_or_else(|_| Client::new()),
            base_url: base_url.into(),
            default_model: model.into(),
            default_language: "zh".into(),
        }
    }

    pub fn with_language(mut self, lang: impl Into<String>) -> Self {
        self.default_language = lang.into();
        self
    }

    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.client = Client::builder()
            .timeout(timeout)
            .build()
            .unwrap_or_else(|_| Client::new());
        self
    }

    /// 转录音频文件。
    ///
    /// - `audio_data`: 音频字节
    /// - `format`: 文件扩展名（"wav" / "mp3" / "ogg" / "webm"）
    /// - `language`: 可选语言代码（ISO-639-1，如 "zh" / "en"）
    pub async fn transcribe(
        &self,
        audio_data: Vec<u8>,
        format: &str,
        language: Option<&str>,
    ) -> Result<String, AudioError> {
        let url = format!("{}/v1/audio/transcriptions", self.base_url.trim_end_matches('/'));

        let file_name = format!("audio.{}", format);
        let part = reqwest::multipart::Part::bytes(audio_data)
            .file_name(file_name)
            .mime_str(&format!("audio/{}", format))
            .map_err(|e| AudioError::Multipart(e.to_string()))?;

        let mut form = reqwest::multipart::Form::new()
            .part("file", part)
            .text("model", self.default_model.clone());
        if let Some(lang) = language {
            form = form.text("language", lang.to_string());
        } else {
            form = form.text("language", self.default_language.clone());
        }

        let resp = self
            .client
            .post(&url)
            .multipart(form)
            .send()
            .await
            .map_err(|e| AudioError::Request(e.to_string()))?;

        if !resp.status().is_success() {
            let status = resp.status().as_u16();
            let text = resp.text().await.unwrap_or_default();
            return Err(AudioError::Status(status, text));
        }

        let v: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| AudioError::Response(e.to_string()))?;

        Ok(v.get("text")
            .and_then(|t| t.as_str())
            .unwrap_or("")
            .to_string())
    }
}

/// TTS Provider — 文本转语音。
///
/// OpenAI 兼容 `/v1/audio/speech` 接口（JSON body）。
pub struct TtsProvider {
    client: Client,
    base_url: String,
    default_model: String,
    default_voice: String,
}

impl TtsProvider {
    pub fn for_endpoint(
        base_url: impl Into<String>,
        model: impl Into<String>,
        voice: impl Into<String>,
    ) -> Self {
        Self {
            client: Client::builder()
                .timeout(Duration::from_secs(60))
                .build()
                .unwrap_or_else(|_| Client::new()),
            base_url: base_url.into(),
            default_model: model.into(),
            default_voice: voice.into(),
        }
    }

    pub fn with_voice(mut self, voice: impl Into<String>) -> Self {
        self.default_voice = voice.into();
        self
    }

    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.client = Client::builder()
            .timeout(timeout)
            .build()
            .unwrap_or_else(|_| Client::new());
        self
    }

    /// 合成语音。
    ///
    /// - `text`: 要合成的文本
    /// - `response_format`: 输出格式（"mp3" / "wav" / "opus" / "pcm"）
    /// 返回音频字节。
    pub async fn synthesize(
        &self,
        text: &str,
        response_format: &str,
    ) -> Result<Vec<u8>, AudioError> {
        let url = format!("{}/v1/audio/speech", self.base_url.trim_end_matches('/'));

        let body = serde_json::json!({
            "model": self.default_model,
            "input": text,
            "voice": self.default_voice,
            "response_format": response_format,
        });

        let resp = self
            .client
            .post(&url)
            .json(&body)
            .send()
            .await
            .map_err(|e| AudioError::Request(e.to_string()))?;

        if !resp.status().is_success() {
            let status = resp.status().as_u16();
            let text = resp.text().await.unwrap_or_default();
            return Err(AudioError::Status(status, text));
        }

        let bytes = resp
            .bytes()
            .await
            .map_err(|e| AudioError::Response(e.to_string()))?;
        Ok(bytes.to_vec())
    }
}