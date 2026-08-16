use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 编程工具配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodingToolConfig {
    /// API 密钥
    pub api_key: Option<String>,
    /// 基础 URL
    pub base_url: Option<String>,
    /// 工作目录
    pub working_dir: Option<String>,
    /// 模型名称
    pub model: Option<String>,
    /// 温度参数
    pub temperature: Option<f32>,
    /// 最大 token 数
    pub max_tokens: Option<u32>,
    /// 超时时间（秒）
    pub timeout_secs: Option<u64>,
    /// 额外配置
    pub extra: HashMap<String, serde_json::Value>,
}

impl Default for CodingToolConfig {
    fn default() -> Self {
        Self {
            api_key: None,
            base_url: None,
            working_dir: None,
            model: None,
            temperature: None,
            max_tokens: None,
            timeout_secs: Some(30),
            extra: HashMap::new(),
        }
    }
}

/// 代码补全请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionRequest {
    /// 文件路径
    pub file_path: String,
    /// 光标位置的代码
    pub prefix: String,
    /// 光标后的代码（可选）
    pub suffix: Option<String>,
    /// 编程语言
    pub language: Option<String>,
    /// 上下文（其他文件内容）
    pub context: Option<Vec<FileContext>>,
}

/// 文件上下文
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileContext {
    pub path: String,
    pub content: String,
}

/// 代码补全响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionResponse {
    /// 补全的代码
    pub completion: String,
    /// 是否有更多补全
    pub has_more: bool,
    /// 其他候选补全
    pub alternatives: Vec<String>,
}

/// 编程对话请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatRequest {
    /// 对话历史
    pub messages: Vec<ChatMessage>,
    /// 当前文件上下文
    pub file_context: Option<FileContext>,
    /// 系统提示
    pub system_prompt: Option<String>,
}

/// 对话消息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: ChatRole,
    pub content: String,
}

/// 对话角色
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ChatRole {
    User,
    Assistant,
    System,
}

/// 对话响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatResponse {
    /// 回复内容
    pub content: String,
    /// 是否有代码变更
    pub has_code_changes: bool,
    /// 代码变更列表
    pub code_changes: Vec<CodeChange>,
}

/// 代码变更
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeChange {
    /// 文件路径
    pub file_path: String,
    /// 变更类型
    pub change_type: ChangeType,
    /// 变更内容
    pub content: String,
    /// 开始行号
    pub start_line: Option<usize>,
    /// 结束行号
    pub end_line: Option<usize>,
}

/// 变更类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ChangeType {
    Create,
    Update,
    Delete,
}

/// 文件读取请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReadFileRequest {
    pub file_path: String,
    pub start_line: Option<usize>,
    pub end_line: Option<usize>,
}

/// 文件写入请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WriteFileRequest {
    pub file_path: String,
    pub content: String,
    pub create_dirs: bool,
}

/// 文件搜索请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchRequest {
    pub query: String,
    pub file_pattern: Option<String>,
    pub directory: Option<String>,
    pub max_results: Option<usize>,
}

/// 搜索结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub file_path: String,
    pub line_number: usize,
    pub line_content: String,
    pub match_score: f32,
}

/// 命令执行请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecRequest {
    pub command: String,
    pub working_dir: Option<String>,
    pub env: Option<HashMap<String, String>>,
    pub timeout_secs: Option<u64>,
}

/// 命令执行响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecResponse {
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
}

/// 代码分析请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalyzeRequest {
    pub file_path: String,
    pub content: Option<String>,
    pub analysis_type: AnalysisType,
}

/// 分析类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum AnalysisType {
    Review,
    Refactor,
    Explain,
    Test,
    Document,
}

/// 代码分析响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalyzeResponse {
    pub summary: String,
    pub issues: Vec<CodeIssue>,
    pub suggestions: Vec<String>,
}

/// 代码问题
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeIssue {
    pub severity: IssueSeverity,
    pub message: String,
    pub line: Option<usize>,
    pub fix: Option<String>,
}

/// 问题严重程度
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum IssueSeverity {
    Error,
    Warning,
    Info,
}
