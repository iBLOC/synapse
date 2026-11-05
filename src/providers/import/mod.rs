//! **外部工具的模型配置** —— 读进来，也写回去。
//!
//! ## 它要解决什么
//!
//! opencode / Claude Code / Codex 各自把「哪个供应商 ＋ 哪把 key」存在
//! **自己的**目录里，格式互不相同（JSON / TOML，字段名也各不相同）。
//! 但它们要的东西是同一件事：**一个能打的供应商**。
//!
//! ⇒ ⇒ 这个模块把那几种形状**读成同一种**（见 ProviderConfig），
//! 也**写回**成它们各自的形状 ⇒ ⇒ 于是「把 opencode 的 key 吸进 shadow」
//! 与「把 shadow 的供应商推给 opencode」是**同一段代码**。
//!
//! ## 三条纪律
//!
//! ① **只读自己认识的文件** —— 路径是**候选列表**，第一个存在的才算；
//!    一个都不存在 ⇒ ⇒ 报告「没装」，**不是**报错。
//! ② **写回只改该改的键** —— 其余条目、其余字段**原样保留** ⇒ ⇒
//!    别人的配置不能被我们改坏 ⇒ ⇒ 写前**备份**、写后**原子替换**。
//! ③ **材料不出现在任何报告里** —— 报告与错误只给**形状**
//!    （长度、来源文件、字段路径），不给材料本身。
//!
//! ## 为什么不叫「同步」
//!
//! 因为它**不同步**：它是在**本机**读另一个工具的盘 ⇄ 写另一个工具的盘。
//! 跨设备同步是**大脑那一层**的事（大脑才是真相源）。

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::providers::config::ProviderConfig;

pub mod claude_code;
pub mod codex;
pub mod opencode;

/// 一个外部工具（今天支持这三个）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ExternalTool {
    OpenCode,
    ClaudeCode,
    Codex,
}

impl ExternalTool {
    /// 稳定 id（给界面 / 协议用）。
    pub fn id(self) -> &'static str {
        match self {
            ExternalTool::OpenCode => "opencode",
            ExternalTool::ClaudeCode => "claude-code",
            ExternalTool::Codex => "codex",
        }
    }

    /// 人看的名字。
    pub fn display_name(self) -> &'static str {
        match self {
            ExternalTool::OpenCode => "opencode",
            ExternalTool::ClaudeCode => "Claude Code",
            ExternalTool::Codex => "Codex",
        }
    }

    /// 全部（**顺序固定** ⇒ ⇒ 界面列表稳定）。
    pub fn all() -> Vec<ExternalTool> {
        vec![
            ExternalTool::OpenCode,
            ExternalTool::ClaudeCode,
            ExternalTool::Codex,
        ]
    }
}

/// 导入 / 写回失败。
///
/// ⚠️ **只 derive Serialize，不 derive Deserialize** —— 它里面有一个
/// `&'static str` 字段 ⇒ ⇒ 反序列化会要求「借出 'static 生命周期」⇒ ⇒
/// 编译不过 ⇒ ⇒ 而这个类型**本来也不该被反序列化**（它是失败，不是载荷）。
#[derive(Debug, Clone, Serialize, thiserror::Error)]
pub enum ImportError {
    #[error("io error on {path}: {message}")]
    Io { path: String, message: String },
    #[error("cannot parse {path}: {message}")]
    Parse { path: String, message: String },
    #[error("{tool} is not installed on this machine (no known config file found)")]
    NotInstalled { tool: &'static str },
    #[error("{0}")]
    Invalid(String),
}

/// 从外部工具读到的一条供应商。
///
/// ⚠️⚠️ **Debug 是手写的** —— 派生版会把 api_key 的**材料**逐字节打进日志
/// ⇒ ⇒ 那正是「密钥进日志」⇒ ⇒ 而这正是明文密钥闸要守的面之一
/// ⇒ ⇒ 派生版会把闸变成摆设。
/// ⇒ ⇒ 这里只报 **id / 来源 / 材料长度**，**一个字的内容都不报**。
#[derive(Clone, Serialize, Deserialize)]
pub struct ImportedProvider {
    /// 外部工具给的标识（opencode: minimax-cn；codex: OPENAI_API_KEY）。
    pub source_id: String,
    /// 统一形状的供应商配置。
    pub config: ProviderConfig,
    /// 材料来自**哪个文件的哪个字段**（**不含材料**）。
    pub origin: String,
}

impl std::fmt::Debug for ImportedProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ImportedProvider")
            .field("source_id", &self.source_id)
            .field("origin", &self.origin)
            // ⚠️ **只报长度** —— 材料一个字都不进日志。
            .field(
                "api_key_len",
                &self.config.api_key.as_ref().map(|k| k.len()),
            )
            .field("base_url", &self.config.base_url)
            .finish()
    }
}

/// 一个工具读到的全部内容。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExternalToolConfig {
    pub tool: ExternalTool,
    /// 实际读到的文件（可能不止一个：凭据文件 ＋ 配置文件）。
    pub files: Vec<PathBuf>,
    /// 读到的供应商（**材料在里面** ⇒ ⇒ 别把整个结构体直接塞进日志）。
    pub providers: Vec<ImportedProvider>,
}

/// 一条写回请求：外部 id → 新材料（None = **不动这一条**）。
pub type WriteUpdate<'a> = (&'a str, Option<&'a str>);

/// 写回的结果。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WriteReport {
    pub tool: ExternalTool,
    /// 实际改动了哪些 id。
    pub changed: Vec<String>,
    /// 备份在哪（可回滚）。
    pub backup: Option<PathBuf>,
    /// 写到哪个文件。
    pub written: PathBuf,
}

/// 这个工具在这台机器上装了没（= 候选路径里有没有一个存在）。
pub fn is_installed(tool: ExternalTool) -> bool {
    match tool {
        ExternalTool::OpenCode => opencode::is_installed(&opencode::detect_paths()),
        ExternalTool::ClaudeCode => claude_code::is_installed(&claude_code::detect_paths()),
        ExternalTool::Codex => codex::is_installed(&codex::detect_paths()),
    }
}

/// 读一个工具的全部供应商配置。
///
/// ⚠️ **没装 ⇒ ⇒ NotInstalled**（不是 IO 错）⇒ ⇒ 调用方能分清
/// 「这台机器没有它」与「它坏了」。
pub fn read(tool: ExternalTool) -> Result<ExternalToolConfig, ImportError> {
    match tool {
        ExternalTool::OpenCode => opencode::read(&opencode::detect_paths()),
        ExternalTool::ClaudeCode => claude_code::read(&claude_code::detect_paths()),
        ExternalTool::Codex => codex::read(&codex::detect_paths()),
    }
}

/// 读**全部装了的**工具（没装的跳过，**不报错**）。
pub fn read_all() -> Vec<ExternalToolConfig> {
    ExternalTool::all()
        .into_iter()
        .filter(|t| is_installed(*t))
        .filter_map(|t| read(t).ok())
        .collect()
}

/// **写回** —— 把新的材料写进那个工具自己的配置。
///
/// ⚠️⚠️ **这是「导出到另一个应用的凭据存储」，不是「把密钥落到自己盘上」**
/// ⇒ ⇒ 那份文件**本来就是那个工具的明文凭据文件** ⇒ ⇒ 我们写进去
/// **不新增**一份明文副本 ⇒ ⇒ 但**其余条目必须原样保留** ⇒ ⇒
/// 且**先备份** ⇒ ⇒ 改坏了能退回去。
pub fn write(tool: ExternalTool, updates: &[WriteUpdate<'_>]) -> Result<WriteReport, ImportError> {
    if updates.is_empty() {
        return Err(ImportError::Invalid("nothing to write".into()));
    }
    match tool {
        ExternalTool::OpenCode => opencode::write(&opencode::detect_paths(), updates),
        ExternalTool::ClaudeCode => claude_code::write(&claude_code::detect_paths(), updates),
        ExternalTool::Codex => codex::write(&codex::detect_paths(), updates),
    }
}

/// 平台相关的路径零件。
pub mod paths {
    use std::path::PathBuf;

    /// 家目录。
    pub fn home() -> Option<PathBuf> {
        std::env::var_os("HOME")
            .or_else(|| std::env::var_os("USERPROFILE"))
            .map(PathBuf::from)
            .filter(|p| !p.as_os_str().is_empty())
    }

    /// XDG 数据目录（Linux / macOS）。
    pub fn data_home() -> Option<PathBuf> {
        std::env::var_os("XDG_DATA_HOME")
            .map(PathBuf::from)
            .or_else(|| home().map(|h| h.join(".local").join("share")))
    }

    /// XDG 配置目录（Linux / macOS）。
    pub fn config_home() -> Option<PathBuf> {
        std::env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .or_else(|| home().map(|h| h.join(".config")))
    }

    /// Windows 的 LOCALAPPDATA。
    pub fn local_app_data() -> Option<PathBuf> {
        std::env::var_os("LOCALAPPDATA").map(PathBuf::from)
    }

    /// Windows 的 APPDATA。
    pub fn app_data() -> Option<PathBuf> {
        std::env::var_os("APPDATA").map(PathBuf::from)
    }
}

/// 第一个**存在**的候选文件（没有 ⇒ None）。
pub(crate) fn first_existing(candidates: &[PathBuf]) -> Option<PathBuf> {
    candidates.iter().find(|p| p.is_file()).cloned()
}

/// 读一个 JSON 文件（不存在 ⇒ None）。
pub(crate) fn read_json(path: &Path) -> Result<Option<serde_json::Value>, ImportError> {
    if !path.is_file() {
        return Ok(None);
    }
    let raw = std::fs::read_to_string(path).map_err(|e| ImportError::Io {
        path: path.display().to_string(),
        message: e.to_string(),
    })?;
    let value = serde_json::from_str(&raw).map_err(|e| ImportError::Parse {
        path: path.display().to_string(),
        message: e.to_string(),
    })?;
    Ok(Some(value))
}

/// **备份 ＋ 原子替换**地写一个 JSON 文件。
///
/// ⚠️⚠️ **参数为什么叫 `credential_doc`（不是 `value`）** —— 这一层写的
///    **整份文档装的就是凭据**（api key / token）⇒ ⇒ **名字必须写实** ⇒ ⇒
///    明文密钥落盘闸（`scripts/check-secret-plaintext.mjs`）靠**名字级**追踪
///    判定 ⇒ ⇒ 参数叫 `value` 的话 ⇒ ⇒ **闸追不到** ⇒ ⇒ 这条导出路径
///    对闸就变成**隐形的** ⇒ ⇒ ⇒ 「写回外部工具的凭据文件」这条漏报 ⇒ ⇒
///    ⇒ ⇒ **改这个参数名之前，先想清楚它在写什么。**
///
/// ⚠️ **为什么先备份**：Windows 上 rename 覆盖已存在文件会失败 ⇒ ⇒
/// 得先删再改名 ⇒ ⇒ **中间有个「文件不存在」的窗口** ⇒ ⇒ 一旦删成功
/// 改名失败，**原文件就没了** ⇒ ⇒ ⇒ 备份是那一下的**退路**。
///
/// ⚠️ **键顺序可能被重排**（serde_json 的 Map 默认按字典序）⇒ ⇒
/// **语义不变**，但那个文件的字面顺序会变 ⇒ ⇒ 那是别人的文件 ⇒ ⇒
/// 记在这里，别当成「我们弄乱了它」。
pub(crate) fn write_json_atomic(
    path: &Path,
    credential_doc: &serde_json::Value,
) -> Result<Option<PathBuf>, ImportError> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| ImportError::Io {
            path: parent.display().to_string(),
            message: e.to_string(),
        })?;
    }

    let content = serde_json::to_string_pretty(credential_doc).map_err(|e| ImportError::Parse {
        path: path.display().to_string(),
        message: e.to_string(),
    })?;

    // 备份（存在才备份 ⇒ ⇒ 第一次写没有「原件」可备）。
    let backup = if path.is_file() {
        let backup = backup_path(path);
        std::fs::copy(path, &backup).map_err(|e| ImportError::Io {
            path: backup.display().to_string(),
            message: e.to_string(),
        })?;
        Some(backup)
    } else {
        None
    };

    let tmp = tmp_path(path);
    std::fs::write(&tmp, content).map_err(|e| ImportError::Io {
        path: tmp.display().to_string(),
        message: e.to_string(),
    })?;
    replace_file(&tmp, path).map_err(|e| ImportError::Io {
        path: path.display().to_string(),
        message: e.to_string(),
    })?;
    Ok(backup)
}

/// 备份文件的名字（auth.json ⇒ auth.json.shadow-backup）。
pub(crate) fn backup_path(path: &Path) -> PathBuf {
    let mut name = path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "config".to_string());
    name.push_str(".shadow-backup");
    path.with_file_name(name)
}

/// 临时文件的名字（与备份**不同名** ⇒ ⇒ 两者不会互相覆盖）。
fn tmp_path(path: &Path) -> PathBuf {
    let mut name = path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "config".to_string());
    name.push_str(".shadow-tmp");
    path.with_file_name(name)
}

/// 用临时文件替换目标文件。
fn replace_file(tmp: &Path, target: &Path) -> std::io::Result<()> {
    #[cfg(windows)]
    {
        // ⚠️ Windows 上 rename 覆盖已存在文件会失败 ⇒ ⇒ 先删再改名。
        // 备份已经保住了原件 ⇒ ⇒ 这一步失败也能退回去。
        if target.exists() {
            std::fs::remove_file(target)?;
        }
        std::fs::rename(tmp, target)
    }
    #[cfg(not(windows))]
    {
        std::fs::rename(tmp, target)
    }
}

/// 从 provider.<id>.options.baseURL 这类结构里取出 **id → 地址**。
pub(crate) fn collect_base_urls(
    doc: &serde_json::Value,
    section: &str,
) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    let Some(map) = doc.get(section).and_then(|v| v.as_object()) else {
        return out;
    };
    for (id, spec) in map {
        let url = spec
            .get("options")
            .and_then(|o| o.get("baseURL"))
            .and_then(|u| u.as_str())
            .or_else(|| spec.get("base_url").and_then(|u| u.as_str()))
            .or_else(|| spec.get("baseURL").and_then(|u| u.as_str()));
        if let Some(url) = url {
            out.insert(id.clone(), url.to_string());
        }
    }
    out
}
