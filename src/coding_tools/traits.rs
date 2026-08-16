use async_trait::async_trait;
use crate::coding_tools::types::*;

/// AI 编程工具统一接口
#[async_trait]
pub trait CodingTool: Send + Sync {
    /// 获取工具名称
    fn name(&self) -> &str;

    /// 获取工具描述
    fn description(&self) -> &str;

    /// 代码补全
    async fn complete(&self, request: CompletionRequest) -> Result<CompletionResponse, CodingToolError>;

    /// 编程对话
    async fn chat(&self, request: ChatRequest) -> Result<ChatResponse, CodingToolError>;

    /// 读取文件
    async fn read_file(&self, request: ReadFileRequest) -> Result<String, CodingToolError>;

    /// 写入文件
    async fn write_file(&self, request: WriteFileRequest) -> Result<(), CodingToolError>;

    /// 搜索代码
    async fn search(&self, request: SearchRequest) -> Result<Vec<SearchResult>, CodingToolError>;

    /// 执行命令
    async fn execute(&self, request: ExecRequest) -> Result<ExecResponse, CodingToolError>;

    /// 代码分析
    async fn analyze(&self, request: AnalyzeRequest) -> Result<AnalyzeResponse, CodingToolError>;
}

/// 编程工具错误
#[derive(Debug, thiserror::Error)]
pub enum CodingToolError {
    #[error("API error: {0}")]
    Api(String),

    #[error("authentication failed: {0}")]
    Auth(String),

    #[error("file not found: {0}")]
    FileNotFound(String),

    #[error("permission denied: {0}")]
    PermissionDenied(String),

    #[error("timeout")]
    Timeout,

    #[error("rate limited")]
    RateLimited,

    #[error("invalid request: {0}")]
    InvalidRequest(String),

    #[error("network error: {0}")]
    Network(String),

    #[error("unknown error: {0}")]
    Unknown(String),
}
