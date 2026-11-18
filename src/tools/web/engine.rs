//! 引擎适配 —— 把「一个真浏览器引擎」接进这一层。
//!
//! ⚠️ **契约**：引擎是**外进程**，本仓**不 vendor** 它的代码
//! ⇒ ⇒ 它的工具名是 **0.x 的、随时会变的** ⇒ ⇒ 所以这一层
//! ① **运行时自省**（`tools/list` 拿清单；缺能力就报缺，**不静默**）；
//! ② **不写死**引擎的参数名（每个引擎自己翻译自己的动作）。
//!
//! 实测（obscura 0.2.7，MCP over HTTP）：
//! - `tools/list` ⇒ **38 个工具**、schema 全文 13,260 字符；
//! - `browser_navigate` 的参数是 `{url, waitUntil}`；
//! - `browser_snapshot` 的参数是 `{max_chars}`；
//! - `browser_extract` 的参数是 `{schema}`（字段名 → CSS 选择器，
//!   `name@attr` 取属性，`name[]` 取数组）。
//!
//! ## ⚠️ 参数名**逐字**对过真 schema（2026-11-16）
//!
//! 之前的映射有四处**对不上** —— 真引擎当场回的错误，**不是猜的**：
//!
//! | 动作 | 引擎要的 | 我们当时传的 | 真引擎逐字回 |
//! |---|---|---|---|
//! | type | `text` | `value` | `Error: Missing text parameter` |
//! | select | `selector`（**不收 ref**） | `ref` | `Error: Missing selector parameter` |
//! | scroll | `direction` | 只有 `selector` | `Scrolled element into view` ⇒ **静默做错事** |
//! | form | `fields`（**数组**） | 对象 | `Error: Missing fields array` |
//!
//! ⚠️ 第三条最毒：**不报错**，只是把「翻页」做成了「把元素滚进视野」
//! ⇒ ⇒ 比报错更坏（失败长得像成功）。
//!
//! ⇒ ⇒ 所以动作模型**不再用「一个 target 字符串 + 一个 value 字符串」**
//! ⇒ ⇒ 目标分成 `ref`／`selector` 两种**互斥**形状
//! ⇒ ⇒ 引擎只收 selector 的动作（select／press）拿到 ref 时**说实话**（报参数错）。
//!
//! ## ⚠️ ref 是**按导航纪元编号**的
//!
//! 实测：每次 `browser_navigate` 之后，ref 从 `e1-1` 变成 `e2-1`、`e3-1`…
//! ⇒ ⇒ 换页之后**旧 ref 立刻失效**（引擎逐字回
//! `unknown ref 'e1-1'; call browser_snapshot first to refresh the ref table`）
//! ⇒ ⇒ 所以 ref 必须来自**当前这一次页面加载**的 snapshot。

use std::collections::HashSet;
use std::sync::atomic::{AtomicI64, Ordering};

use async_trait::async_trait;
use serde_json::{json, Map, Value};
use thiserror::Error;

/// 读页面的几种方式。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReadMode {
    /// 可读正文（markdown）
    Text,
    /// 快照（URL／标题／元素引用／可见正文）
    Snapshot,
    /// 链接清单
    Links,
    /// 按选择器抽结构化字段
    Extract,
    /// 控制台消息
    Console,
    /// 网络请求清单
    Network,
}

/// 交互动作（**有外部副作用** ⇒ ⇒ 默认不许）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActKind {
    Click,
    Fill,
    Type,
    Press,
    Select,
    Scroll,
    /// 整表填写（会提交 ⇒ ⇒ 最危险的一个）
    Form,
}

/// 元素目标 —— **两种互斥的形状**，不许混着猜。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Target {
    /// 快照给的元素引用（形如 `e4-1`）—— **按导航纪元编号，换页即过期**。
    Ref(String),
    /// CSS 选择器。
    Selector(String),
}

/// 翻页方向（引擎的 `browser_scroll.direction`）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScrollDirection {
    Top,
    Bottom,
    Up,
    Down,
    Left,
    Right,
}

impl ScrollDirection {
    pub fn as_str(&self) -> &'static str {
        match self {
            ScrollDirection::Top => "top",
            ScrollDirection::Bottom => "bottom",
            ScrollDirection::Up => "up",
            ScrollDirection::Down => "down",
            ScrollDirection::Left => "left",
            ScrollDirection::Right => "right",
        }
    }

    /// 认不得就返回 `None` ⇒ ⇒ 门面报「参数不对」，**不**自己挑一个。
    pub fn parse(raw: &str) -> Option<Self> {
        match raw {
            "top" => Some(ScrollDirection::Top),
            "bottom" => Some(ScrollDirection::Bottom),
            "up" => Some(ScrollDirection::Up),
            "down" => Some(ScrollDirection::Down),
            "left" => Some(ScrollDirection::Left),
            "right" => Some(ScrollDirection::Right),
            _ => None,
        }
    }
}

/// 表单字段的填法（引擎的 `fields[].type`）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FieldKind {
    Text,
    Check,
    Uncheck,
    Select,
}

impl FieldKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            FieldKind::Text => "text",
            FieldKind::Check => "check",
            FieldKind::Uncheck => "uncheck",
            FieldKind::Select => "select",
        }
    }

    pub fn parse(raw: &str) -> Option<Self> {
        match raw {
            "text" => Some(FieldKind::Text),
            "check" => Some(FieldKind::Check),
            "uncheck" => Some(FieldKind::Uncheck),
            "select" => Some(FieldKind::Select),
            _ => None,
        }
    }
}

/// 一个表单字段。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FieldSpec {
    /// 字段落在哪个元素上。
    pub target: Target,
    /// 填进去的值。
    pub value: String,
    /// 填法（默认 `text`）。
    pub kind: FieldKind,
}

/// 一次动作。
#[derive(Debug, Clone)]
pub enum WebAction {
    Navigate {
        url: String,
        wait_until: Option<String>,
    },
    Read {
        mode: ReadMode,
        schema: Option<Value>,
        max_chars: Option<usize>,
    },
    Act {
        kind: ActKind,
        /// 动作落在哪个元素上（`click`／`fill`／`type` 必填）。
        target: Option<Target>,
        /// `fill`／`type`／`select` 的值。
        value: Option<String>,
        /// `press` 的键。
        key: Option<String>,
        /// `form` 的字段（**数组**）。
        fields: Option<Vec<FieldSpec>>,
        /// `scroll` 的方向。
        direction: Option<ScrollDirection>,
        /// `scroll` 的像素数。
        amount: Option<u32>,
        /// `form` 填完之后要点的东西（会**提交** ⇒ ⇒ 最危险）。
        submit: Option<Target>,
    },
}

/// 引擎那边出的错。
#[derive(Debug, Error)]
pub enum WebError {
    #[error("engine transport failed: {0}")]
    Transport(String),
    #[error("engine refused the call: {0}")]
    Engine(String),
    #[error("engine does not expose '{0}'")]
    CapabilityMissing(String),
    /// 这一层的动作**没法**翻译成引擎的参数（不是引擎的错，是我们的）。
    #[error("cannot build the engine call: {0}")]
    BadArgument(String),
}

/// 一个「能取网页」的引擎。
///
/// ⚠️ `capabilities()` 是**运行时自省**：门面在调任何动作前先问它
/// 有没有这个工具 ⇒ ⇒ 上游改名字时，报的是「缺能力」，**不是**静默失败。
#[async_trait]
pub trait WebEngine: Send + Sync + std::fmt::Debug {
    /// 引擎暴露的工具名清单。
    async fn capabilities(&self) -> Result<Vec<String>, WebError>;

    /// 这个动作对应引擎的哪个工具（`None` = 这个引擎不支持这个动作）。
    fn tool_for(&self, action: &WebAction) -> Option<String>;

    /// 执行动作（引擎自己把动作翻译成自己的参数名）。
    async fn invoke(&self, action: WebAction) -> Result<String, WebError>;
}

/// 动作 → 引擎的工具名（**纯函数** ⇒ ⇒ 判据能直接钉它）。
pub fn obscura_tool(action: &WebAction) -> Option<&'static str> {
    let name = match action {
        WebAction::Navigate { .. } => "browser_navigate",
        WebAction::Read { mode, .. } => match mode {
            ReadMode::Text => "browser_markdown",
            ReadMode::Snapshot => "browser_snapshot",
            ReadMode::Links => "browser_links",
            ReadMode::Extract => "browser_extract",
            ReadMode::Console => "browser_console_messages",
            ReadMode::Network => "browser_network_requests",
        },
        WebAction::Act { kind, .. } => match kind {
            ActKind::Click => "browser_click",
            ActKind::Fill => "browser_fill",
            ActKind::Type => "browser_type",
            ActKind::Press => "browser_press_key",
            ActKind::Select => "browser_select_option",
            ActKind::Scroll => "browser_scroll",
            ActKind::Form => "browser_fill_form",
        },
    };
    Some(name)
}

/// 动作 → 引擎的实参（**纯函数** ⇒ ⇒ 判据能逐字钉住参数名）。
///
/// ⚠️ 这一层**只**知道 obscura 的参数名 ⇒ ⇒ 换引擎就换这一处。
pub fn obscura_args(action: &WebAction) -> Result<Value, WebError> {
    let mut m = Map::new();
    match action {
        WebAction::Navigate { url, wait_until } => {
            m.insert("url".into(), json!(url));
            if let Some(wait) = wait_until {
                m.insert("waitUntil".into(), json!(wait));
            }
        }
        WebAction::Read {
            mode,
            schema,
            max_chars,
        } => match mode {
            ReadMode::Snapshot => {
                m.insert("max_chars".into(), json!(max_chars.unwrap_or(4000)));
            }
            ReadMode::Extract => {
                m.insert("schema".into(), schema.clone().unwrap_or_else(|| json!({})));
            }
            _ => {}
        },
        WebAction::Act {
            kind,
            target,
            value,
            key,
            fields,
            direction,
            amount,
            submit,
        } => match kind {
            ActKind::Click => {
                let t = target
                    .as_ref()
                    .ok_or_else(|| WebError::BadArgument("click needs a target".into()))?;
                put_target(&mut m, t);
            }
            ActKind::Fill => {
                let t = target
                    .as_ref()
                    .ok_or_else(|| WebError::BadArgument("fill needs a target".into()))?;
                put_target(&mut m, t);
                m.insert("value".into(), json!(value.clone().unwrap_or_default()));
            }
            ActKind::Type => {
                let t = target
                    .as_ref()
                    .ok_or_else(|| WebError::BadArgument("type needs a target".into()))?;
                put_target(&mut m, t);
                // ⚠️ 引擎的参数叫 **`text`**（不是 `value`）
                // ⇒ ⇒ 真引擎逐字回「Error: Missing text parameter」。
                m.insert("text".into(), json!(value.clone().unwrap_or_default()));
            }
            ActKind::Press => {
                let t = target
                    .as_ref()
                    .ok_or_else(|| WebError::BadArgument("press needs a target".into()))?;
                // ⚠️ 引擎的 `browser_press_key` **只收 selector**
                // ⇒ ⇒ 拿 ref 来 ⇒ 说实话（不把 ref 硬塞进 selector）。
                put_selector_only(&mut m, t, "browser_press_key")?;
                m.insert("key".into(), json!(key.clone().unwrap_or_default()));
            }
            ActKind::Select => {
                let t = target
                    .as_ref()
                    .ok_or_else(|| WebError::BadArgument("select needs a target".into()))?;
                // ⚠️ 引擎的 `browser_select_option` **只收 selector**
                // ⇒ ⇒ 传 ref 时真引擎逐字回「Error: Missing selector parameter」。
                put_selector_only(&mut m, t, "browser_select_option")?;
                m.insert("value".into(), json!(value.clone().unwrap_or_default()));
            }
            ActKind::Scroll => {
                // ⚠️ 引擎的 `direction` **不给就变成「把元素滚进视野」**
                // ⇒ ⇒ 这里如实说清：没方向就没有「翻页」这回事。
                match direction {
                    Some(d) => {
                        m.insert("direction".into(), json!(d.as_str()));
                    }
                    None => {
                        return Err(WebError::BadArgument(
                            "scroll needs a direction (top/bottom/up/down/left/right)".into(),
                        ));
                    }
                }
                if let Some(a) = amount {
                    m.insert("amount".into(), json!(a));
                }
                if let Some(t) = target {
                    put_target(&mut m, t);
                }
            }
            ActKind::Form => {
                let fs = fields
                    .as_ref()
                    .ok_or_else(|| WebError::BadArgument("fill form needs 'fields'".into()))?;
                let mut arr = Vec::with_capacity(fs.len());
                for f in fs {
                    let mut fm = Map::new();
                    put_target(&mut fm, &f.target);
                    fm.insert("value".into(), json!(f.value));
                    fm.insert("type".into(), json!(f.kind.as_str()));
                    arr.push(Value::Object(fm));
                }
                // ⚠️ 引擎要的是**数组** ⇒ ⇒ 传对象时真引擎逐字回
                // 「Error: Missing fields array」。
                m.insert("fields".into(), Value::Array(arr));
                if let Some(s) = submit {
                    match s {
                        Target::Ref(r) => {
                            m.insert("submit_ref".into(), json!(r));
                        }
                        Target::Selector(sel) => {
                            m.insert("submit_selector".into(), json!(sel));
                        }
                    }
                }
            }
        },
    }
    Ok(Value::Object(m))
}

/// 目标：`ref` 走 `ref`，`selector` 走 `selector`（**互斥，不猜**）。
fn put_target(m: &mut Map<String, Value>, target: &Target) {
    match target {
        Target::Ref(r) => {
            m.insert("ref".into(), json!(r));
        }
        Target::Selector(s) => {
            m.insert("selector".into(), json!(s));
        }
    }
}

/// 只收 selector 的动作（`select`／`press`）⇒ ⇒ 拿到 ref 就**说实话**。
fn put_selector_only(
    m: &mut Map<String, Value>,
    target: &Target,
    tool: &str,
) -> Result<(), WebError> {
    match target {
        Target::Selector(s) => {
            m.insert("selector".into(), json!(s));
            Ok(())
        }
        Target::Ref(_) => Err(WebError::BadArgument(format!(
            "{tool} only takes a CSS selector — pass 'selector', not 'ref'"
        ))),
    }
}

/// obscura 的引擎适配（MCP over HTTP）。
///
/// ⚠️ **为什么走 HTTP 而不是 stdio**：obscura 的 MCP stdio 走
/// **按行 JSON**（MCP 规范），而本仓 `synapse::mcp::StdioMcpClient`
/// 走的是 LSP 式 `Content-Length` 分帧 ⇒ ⇒ 两者**对不上**
/// （实测：按行 JSON 有回包，Content-Length 6 秒无回包）。
/// ⇒ ⇒ 这里走 HTTP（已实测通）；要补 stdio 的话得另加一个按行分帧的客户端。
#[derive(Debug)]
pub struct ObscuraEngine {
    endpoint: String,
    token: Option<String>,
    client: reqwest::Client,
    caps: tokio::sync::RwLock<Option<HashSet<String>>>,
    next_id: AtomicI64,
}

impl ObscuraEngine {
    /// `endpoint` 是 MCP 的 HTTP 端点（形如 `http://127.0.0.1:3111/mcp`）。
    pub fn new(endpoint: impl Into<String>, token: Option<String>) -> Self {
        Self {
            endpoint: endpoint.into(),
            token,
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(120))
                // ⚠️ **不许走代理** —— MCP 端点在**回环**上。
                // 实测（Windows）：系统代理开着（Clash：127.0.0.1:7897）时，
                // reqwest 会把回环端点 **也发给代理**
                // ⇒ ⇒ 代理够不到那个端口 ⇒ ⇒ 回 **502 Bad Gateway**（空 body）。
                // （.NET 的 HttpClient 默认绕过本地地址，reqwest **不**绕 ⇒ ⇒
                // 同一台机器上 PowerShell 探针能通、Rust 引擎却 502 —— 就是这个差。）
                .no_proxy()
                .build()
                .unwrap_or_default(),
            caps: tokio::sync::RwLock::new(None),
            next_id: AtomicI64::new(1),
        }
    }

    async fn rpc(&self, method: &str, params: Value) -> Result<Value, WebError> {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let body = json!({"jsonrpc": "2.0", "id": id, "method": method, "params": params});
        let mut req = self
            .client
            .post(&self.endpoint)
            .header("content-type", "application/json")
            .header("accept", "application/json, text/event-stream")
            .body(
                serde_json::to_vec(&body).map_err(|e| WebError::Transport(e.to_string()))?,
            );
        if let Some(token) = &self.token {
            req = req.bearer_auth(token);
        }
        let resp = req
            .send()
            .await
            .map_err(|e| WebError::Transport(e.to_string()))?;
        let status = resp.status();
        let text = resp
            .text()
            .await
            .map_err(|e| WebError::Transport(e.to_string()))?;
        if !status.is_success() {
            let head: String = text.chars().take(200).collect();
            return Err(WebError::Transport(format!("HTTP {status}: {head}")));
        }
        let value =
            parse_jsonrpc(&text).ok_or_else(|| WebError::Transport("unparsable response".into()))?;
        if let Some(err) = value.get("error") {
            return Err(WebError::Engine(err.to_string()));
        }
        value
            .get("result")
            .cloned()
            .ok_or_else(|| WebError::Transport("response carried no result".into()))
    }

    async fn fetch_capabilities(&self) -> Result<Vec<String>, WebError> {
        let result = self.rpc("tools/list", json!({})).await?;
        let tools = result
            .get("tools")
            .and_then(|t| t.as_array())
            .ok_or_else(|| WebError::Transport("tools/list carried no tools array".into()))?;
        Ok(tools
            .iter()
            .filter_map(|t| t.get("name").and_then(|n| n.as_str()).map(|s| s.to_string()))
            .collect())
    }

    async fn call_tool(&self, tool: &str, args: Value) -> Result<String, WebError> {
        let result = self
            .rpc("tools/call", json!({"name": tool, "arguments": args}))
            .await?;
        let is_error = result
            .get("isError")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let text = result
            .get("content")
            .and_then(|c| c.as_array())
            .and_then(|arr| arr.first())
            .and_then(|c| c.get("text"))
            .and_then(|t| t.as_str())
            .unwrap_or("")
            .to_string();
        if is_error {
            return Err(WebError::Engine(text));
        }
        Ok(text)
    }
}

#[async_trait]
impl WebEngine for ObscuraEngine {
    async fn capabilities(&self) -> Result<Vec<String>, WebError> {
        {
            let guard = self.caps.read().await;
            if let Some(set) = guard.as_ref() {
                return Ok(set.iter().cloned().collect());
            }
        }
        let list = self.fetch_capabilities().await?;
        let set: HashSet<String> = list.into_iter().collect();
        *self.caps.write().await = Some(set.clone());
        Ok(set.into_iter().collect())
    }

    fn tool_for(&self, action: &WebAction) -> Option<String> {
        obscura_tool(action).map(str::to_string)
    }

    async fn invoke(&self, action: WebAction) -> Result<String, WebError> {
        let tool = obscura_tool(&action)
            .ok_or_else(|| WebError::CapabilityMissing("unknown action".into()))?;
        let args = obscura_args(&action)?;
        self.call_tool(tool, args).await
    }
}

/// 解析 MCP 响应：普通 JSON，或 SSE（`data: {…}`）。
fn parse_jsonrpc(text: &str) -> Option<Value> {
    let trimmed = text.trim_start();
    if trimmed.starts_with('{') || trimmed.starts_with('[') {
        return serde_json::from_str(trimmed).ok();
    }
    for line in text.lines() {
        if let Some(rest) = line.strip_prefix("data:")
            && let Ok(value) = serde_json::from_str::<Value>(rest.trim())
        {
            return Some(value);
        }
    }
    None
}
