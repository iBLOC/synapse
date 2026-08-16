//! AI 编程工具客户端模块
//!
//! 提供统一的接口来调用各种 AI 编程工具的能力：
//! - 代码补全与生成
//! - 编程对话
//! - 文件操作
//! - 命令执行
//! - 代码分析
//!
//! 支持的工具：
//! - OpenCode
//! - Codex CLI (OpenAI)
//! - Claude Code (Anthropic)
//! - Pi

pub mod traits;
pub mod types;
pub mod opencode;
pub mod codex;
pub mod claude_code;
pub mod pi;

pub use traits::CodingTool;
pub use types::*;
pub use opencode::OpenCodeClient;
pub use codex::CodexClient;
pub use claude_code::ClaudeCodeClient;
pub use pi::PiClient;

use std::sync::Arc;

/// 编程工具工厂
pub struct CodingToolFactory;

impl CodingToolFactory {
    /// 创建指定的编程工具客户端
    pub fn create(tool_type: CodingToolType, config: CodingToolConfig) -> Arc<dyn CodingTool> {
        match tool_type {
            CodingToolType::OpenCode => Arc::new(OpenCodeClient::new(config)),
            CodingToolType::Codex => Arc::new(CodexClient::new(config)),
            CodingToolType::ClaudeCode => Arc::new(ClaudeCodeClient::new(config)),
            CodingToolType::Pi => Arc::new(PiClient::new(config)),
        }
    }
}

/// 编程工具类型
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CodingToolType {
    OpenCode,
    Codex,
    ClaudeCode,
    Pi,
}

impl CodingToolType {
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "opencode" => Some(Self::OpenCode),
            "codex" => Some(Self::Codex),
            "claude_code" | "claudecode" => Some(Self::ClaudeCode),
            "pi" => Some(Self::Pi),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_coding_tool_type_from_str() {
        assert_eq!(CodingToolType::from_str("opencode"), Some(CodingToolType::OpenCode));
        assert_eq!(CodingToolType::from_str("codex"), Some(CodingToolType::Codex));
        assert_eq!(CodingToolType::from_str("claude_code"), Some(CodingToolType::ClaudeCode));
        assert_eq!(CodingToolType::from_str("claudecode"), Some(CodingToolType::ClaudeCode));
        assert_eq!(CodingToolType::from_str("pi"), Some(CodingToolType::Pi));
        assert_eq!(CodingToolType::from_str("unknown"), None);
    }
}
