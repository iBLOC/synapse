//! web 能力面 —— 把「一个真浏览器引擎」收敛成**三个自有名**的工具。
//!
//! # 为什么不是把引擎的工具原样挂出去
//!
//! ❶ **名字会撞**：端脑给模型看的是**裸名**（`mcp:<server>:<tool>` 取最后
//! 一段）⇒ ⇒ 引擎自带的 `browser_navigate` **直接撞** Playwright MCP
//! （桌面端 `skills.json` 已内置 playwright 技能）⇒ ⇒ 撞名时端脑
//! 「第一个 provider 赢」，只 `warn` 不报错 ⇒ ⇒ 静默走错引擎。
//! ❷ **上下文预算**：端脑每轮把 `exposed_tools` **全量**塞进 tools 数组
//! （本体工具已占 11 个）⇒ ⇒ 引擎那 **38 个工具**（schema 全文 13,260 字符）
//! 加进来是预算事故。
//! ❸ **白名单会烂**：端脑的 `ToolPolicy.allowed` 是**硬编码集合**，且有一
//! 条测试要求「白名单 ⊆ 真 server 的工具清单」⇒ ⇒ 引擎是 0.x、上游改个
//! 工具名，白名单当场烂掉。
//!
//! ⇒ ⇒ 所以：**引擎在**外进程**，能力在这一层**，工具名是**我们自己的**。
//!
//! # 这一层的四条硬规矩
//!
//! - **门**（[`policy`］）：字面私网／环回／link-local／云元数据**一律拒**；
//!   fake-IP 机器上**只**放开 benchmark 段（探测，不是假设）。
//! - **只读优先**：交互动作（点击／填表／提交）**默认关**（
//!   `WebPolicy::allow_interactive`）—— 端脑那侧还没有客户端确认流
//!   ⇒ ⇒ `NeedsConfirmation` 等于被拒 ⇒ ⇒ 默认只给只读动作。
//! - **不静默**：引擎缺能力 ⇒ 报「缺能力」；引擎挂 ⇒ 报错；**不**返回空成功。
//! - **截断**：输出有上限，超了**带标记**截断（不按字节硬切 UTF-8）。

pub mod engine;
pub mod engine_binary;
pub mod policy;
pub mod server;

#[cfg(test)]
mod tests;

use std::sync::Arc;

use async_trait::async_trait;
use serde_json::{json, Value};
use uuid::Uuid;

use crate::tools::{Tool, ToolError, ToolMetadata, ToolOutput, ToolRegistry};

use engine::{
    ActKind, FieldKind, FieldSpec, ReadMode, ScrollDirection, Target, WebAction, WebEngine, WebError,
};
use policy::check_url;

/// 这一层的策略。
#[derive(Debug, Clone)]
pub struct WebPolicy {
    /// 允许 198.18.0.0/15（fake-IP 机器上**必须**开，否则一个站都打不开）。
    ///
    /// ⚠️ 只在**探测到 fake-IP 解析器**时为真（见
    /// [`policy::detect_fakeip_resolver`]）⇒ ⇒ 它只放开 benchmark 段。
    pub allow_fakeip_range: bool,

    /// 允许交互动作（点击／填表／提交）—— 默认**关**。
    pub allow_interactive: bool,

    /// 单次输出上限（字节）。
    pub max_output_bytes: usize,

    /// snapshot 的正文预算（字符）。
    pub snapshot_chars: usize,
}

impl Default for WebPolicy {
    fn default() -> Self {
        Self {
            allow_fakeip_range: false,
            allow_interactive: false,
            max_output_bytes: 8192,
            snapshot_chars: 4000,
        }
    }
}

impl WebPolicy {
    /// 按这台机器的**实际**情况定策略（探测，不是假设）。
    pub fn detect() -> Self {
        Self {
            allow_fakeip_range: policy::detect_fakeip_resolver(),
            ..Self::default()
        }
    }
}

/// 一个会话（一个引擎 + 一份策略）。
#[derive(Debug)]
pub struct WebSession {
    engine: Arc<dyn WebEngine>,
    policy: WebPolicy,
}

impl WebSession {
    pub fn new(engine: Arc<dyn WebEngine>, policy: WebPolicy) -> Self {
        Self { engine, policy }
    }

    pub fn policy(&self) -> &WebPolicy {
        &self.policy
    }

    pub fn engine(&self) -> &Arc<dyn WebEngine> {
        &self.engine
    }

    /// 跑一个动作：查能力 → 调用 → 截断 → 记账。
    async fn run(&self, action: WebAction, interactive: bool) -> Result<String, ToolError> {
        if interactive && !self.policy.allow_interactive {
            tracing::warn!(
                "web: interactive action refused by policy (allow_interactive=false)"
            );
            return Err(ToolError::NotAuthorized(
                "interactive web actions are disabled by policy".into(),
            ));
        }
        let tool = self
            .engine
            .tool_for(&action)
            .ok_or_else(|| ToolError::InvalidParameters("unsupported web action".into()))?;
        // 运行时自省：引擎没有这个工具 ⇒ 报缺能力，**不**静默失败
        let caps = self
            .engine
            .capabilities()
            .await
            .map_err(|e| ToolError::ExternalServiceError(e.to_string()))?;
        if !caps.iter().any(|c| c == &tool) {
            tracing::warn!(tool = %tool, "web: engine lacks the capability");
            return Err(ToolError::ToolDisabled(format!(
                "web engine does not expose '{tool}'"
            )));
        }
        let out = self
            .engine
            .invoke(action)
            .await
            .map_err(|e| match e {
                WebError::CapabilityMissing(name) => {
                    ToolError::ToolDisabled(format!("web engine does not expose '{name}'"))
                }
                // ⚠️ 参数对不上 ⇒ ⇒ 是**我们的**问题 ⇒ ⇒ 说「参数不对」，
                // 别把它说成「引擎挂了」（失败要长得像失败）。
                WebError::BadArgument(msg) => ToolError::InvalidParameters(msg),
                other => ToolError::ExternalServiceError(other.to_string()),
            })?;
        let truncated = truncate_bytes(&out, self.policy.max_output_bytes);
        tracing::info!(
            tool = %tool,
            bytes = truncated.len(),
            "web: action ok"
        );
        Ok(truncated)
    }

    /// 门：这条 URL 允许去取吗？
    fn guard_url(&self, url: &str) -> Result<(), ToolError> {
        check_url(url, self.policy.allow_fakeip_range).map_err(|e| {
            tracing::warn!(url = %url, error = %e, "web: url refused by policy");
            ToolError::NotAuthorized(format!("url refused by the web policy: {e}"))
        })
    }
}

/// 按字节上限截断，**不切坏 UTF-8**。
pub fn truncate_bytes(text: &str, max: usize) -> String {
    if text.len() <= max {
        return text.to_string();
    }
    let mut end = max;
    while end > 0 && !text.is_char_boundary(end) {
        end -= 1;
    }
    format!("{}\n<truncated {} bytes>", &text[..end], text.len() - end)
}

/// `web_fetch` —— 打开一个 URL 并取回可读内容（**只读**）。
#[derive(Debug)]
pub struct WebFetchTool {
    session: Arc<WebSession>,
}

impl WebFetchTool {
    pub fn new(session: Arc<WebSession>) -> Self {
        Self { session }
    }
}

#[async_trait]
impl Tool for WebFetchTool {
    fn name(&self) -> &str {
        "web_fetch"
    }

    fn description(&self) -> &str {
        "Open a URL in the web engine and return its readable content (read-only)."
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "url": { "type": "string", "description": "http(s) URL to open" },
                "mode": {
                    "type": "string",
                    "enum": ["text", "snapshot", "links", "extract"],
                    "default": "text",
                    "description": "What to return: readable text, a snapshot, links, or structured fields"
                },
                "schema": {
                    "type": "object",
                    "description": "For mode=extract: field name -> CSS selector ('name@attr' reads an attribute, 'name[]' reads an array)"
                },
                "wait_until": {
                    "type": "string",
                    "enum": ["load", "domcontentloaded", "networkidle0"],
                    "description": "Navigation wait condition (default: load)"
                }
            },
            "required": ["url"]
        })
    }

    async fn execute(&self, input: Value) -> Result<ToolOutput, ToolError> {
        let url = input
            .get("url")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::InvalidParameters("missing 'url'".into()))?;
        let mode = read_mode(input.get("mode"))?;
        let schema = input.get("schema").cloned();
        let wait_until = input
            .get("wait_until")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        self.session.guard_url(url)?;
        let nav = self
            .session
            .run(
                WebAction::Navigate {
                    url: url.to_string(),
                    wait_until,
                },
                false,
            )
            .await?;
        let content = self
            .session
            .run(
                WebAction::Read {
                    mode,
                    schema,
                    max_chars: Some(self.session.policy.snapshot_chars),
                },
                false,
            )
            .await?;
        Ok(ToolOutput::success(
            Uuid::new_v4(),
            format!("{nav}\n\n{content}"),
        ))
    }
}

/// `web_read` —— 读**当前页**（不导航）。
#[derive(Debug)]
pub struct WebReadTool {
    session: Arc<WebSession>,
}

impl WebReadTool {
    pub fn new(session: Arc<WebSession>) -> Self {
        Self { session }
    }
}

#[async_trait]
impl Tool for WebReadTool {
    fn name(&self) -> &str {
        "web_read"
    }

    fn description(&self) -> &str {
        "Read the page the web engine is currently on (read-only)."
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "mode": {
                    "type": "string",
                    "enum": ["text", "snapshot", "links", "extract", "console", "network"],
                    "default": "text",
                    "description": "What to read"
                },
                "schema": {
                    "type": "object",
                    "description": "For mode=extract: field name -> CSS selector"
                },
                "max_chars": { "type": "integer", "description": "Text budget for mode=snapshot" }
            }
        })
    }

    async fn execute(&self, input: Value) -> Result<ToolOutput, ToolError> {
        let mode = read_mode(input.get("mode"))?;
        let schema = input.get("schema").cloned();
        let max_chars = input
            .get("max_chars")
            .and_then(|v| v.as_u64())
            .map(|v| v as usize)
            .or(Some(self.session.policy.snapshot_chars));
        let content = self
            .session
            .run(
                WebAction::Read {
                    mode,
                    schema,
                    max_chars,
                },
                false,
            )
            .await?;
        Ok(ToolOutput::success(Uuid::new_v4(), content))
    }
}

/// `web_act` —— 交互动作（**默认被策略关着**）。
#[derive(Debug)]
pub struct WebActTool {
    session: Arc<WebSession>,
}

impl WebActTool {
    pub fn new(session: Arc<WebSession>) -> Self {
        Self { session }
    }
}

#[async_trait]
impl Tool for WebActTool {
    fn name(&self) -> &str {
        "web_act"
    }

    fn description(&self) -> &str {
        "Interact with the current page (click / fill / type / press / select / scroll / fill form). Needs explicit approval; disabled unless the policy allows it. Element refs come from the latest snapshot and expire on the next navigation — re-read the page instead of reusing an old ref."
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "enum": ["click", "fill", "type", "press", "select", "scroll", "form"],
                    "description": "What to do"
                },
                "ref": {
                    "type": "string",
                    "description": "Element ref from the latest snapshot (e.g. 'e4-1'). Refs are numbered per page load — they expire on the next navigation."
                },
                "selector": {
                    "type": "string",
                    "description": "CSS selector. Required for select / press — the engine accepts no ref there."
                },
                "target": {
                    "type": "string",
                    "description": "Convenience: an element ref OR a CSS selector, guessed from its shape. Prefer 'ref' / 'selector'."
                },
                "value": { "type": "string", "description": "Value for fill / type / select" },
                "key": { "type": "string", "description": "Key for press (e.g. 'Enter')" },
                "direction": {
                    "type": "string",
                    "enum": ["top", "bottom", "up", "down", "left", "right"],
                    "description": "For action=scroll: which way to scroll"
                },
                "amount": {
                    "type": "integer",
                    "description": "For action=scroll: pixels to scroll (default: one viewport)"
                },
                "fields": {
                    "type": "array",
                    "description": "For action=form: the fields to fill (an ARRAY, not a map)",
                    "items": {
                        "type": "object",
                        "properties": {
                            "ref": { "type": "string" },
                            "selector": { "type": "string" },
                            "value": { "type": "string" },
                            "type": {
                                "type": "string",
                                "enum": ["text", "check", "uncheck", "select"]
                            }
                        }
                    }
                },
                "submit_ref": {
                    "type": "string",
                    "description": "For action=form: element to click after filling (this SUBMITS)"
                },
                "submit_selector": {
                    "type": "string",
                    "description": "For action=form: CSS selector to click after filling (this SUBMITS)"
                }
            },
            "required": ["action"]
        })
    }

    fn metadata(&self) -> ToolMetadata {
        ToolMetadata::new(self.name(), self.description())
            .with_schema(self.input_schema())
            .with_write()
    }

    async fn execute(&self, input: Value) -> Result<ToolOutput, ToolError> {
        let kind = act_kind(input.get("action"))?;
        let target = parse_target(&input)?;
        let value = input.get("value").and_then(|v| v.as_str()).map(|s| s.to_string());
        let key = input.get("key").and_then(|v| v.as_str()).map(|s| s.to_string());
        let fields = parse_fields(input.get("fields"))?;
        let direction = parse_direction(input.get("direction"))?;
        // ⚠️ 「翻页没有方向」是**我们自己的工具契约** ⇒ ⇒ 门面就拒（不靠引擎拒）
        // ⇒ ⇒ 引擎层那道拒是第二道防线（防别的引擎默默做别的事）。
        if kind == ActKind::Scroll && direction.is_none() {
            return Err(ToolError::InvalidParameters(
                "action=scroll needs a 'direction' (top/bottom/up/down/left/right)".into(),
            ));
        }
        let amount = input
            .get("amount")
            .and_then(|v| v.as_u64())
            .map(|v| v as u32);
        let submit = parse_optional_target(
            input.get("submit_ref").and_then(|v| v.as_str()),
            input.get("submit_selector").and_then(|v| v.as_str()),
        );
        let out = self
            .session
            .run(
                WebAction::Act {
                    kind,
                    target,
                    value,
                    key,
                    fields,
                    direction,
                    amount,
                    submit,
                },
                true,
            )
            .await?;
        Ok(ToolOutput::success(Uuid::new_v4(), out))
    }
}

/// 把三个工具都注册进一个 registry。
pub fn register_all(registry: &ToolRegistry, session: Arc<WebSession>) {
    registry.register(WebFetchTool::new(session.clone()));
    registry.register(WebReadTool::new(session.clone()));
    registry.register(WebActTool::new(session));
}

fn read_mode(value: Option<&Value>) -> Result<ReadMode, ToolError> {
    match value.and_then(|v| v.as_str()).unwrap_or("text") {
        "text" | "markdown" => Ok(ReadMode::Text),
        "snapshot" => Ok(ReadMode::Snapshot),
        "links" => Ok(ReadMode::Links),
        "extract" => Ok(ReadMode::Extract),
        "console" => Ok(ReadMode::Console),
        "network" => Ok(ReadMode::Network),
        other => Err(ToolError::InvalidParameters(format!(
            "unsupported mode: {other}"
        ))),
    }
}

/// 目标：**两种互斥的形状**。
/// - 显式 `selector` → 选择器；
/// - 显式 `ref` → 引用；
/// - `target` → **猜**（像 CSS 选择器就当选择器，否则当引用）。
fn parse_target(input: &Value) -> Result<Option<Target>, ToolError> {
    let selector = input.get("selector").and_then(|v| v.as_str());
    let reference = input.get("ref").and_then(|v| v.as_str());
    let target = input.get("target").and_then(|v| v.as_str());
    let picked = match (selector, reference, target) {
        (Some(s), _, _) => Some(Target::Selector(s.to_string())),
        (None, Some(r), _) => Some(Target::Ref(r.to_string())),
        (None, None, Some(t)) => Some(guess_target(t)),
        _ => None,
    };
    Ok(picked)
}

/// `target` 只有一个值 ⇒ ⇒ 只能**猜**它是引用还是选择器。
/// 猜的规则：带 CSS 记号的当选择器，否则当引用（引用的形状是 `e4-1`）。
/// ⇒ ⇒ 裸标签名（`a`／`body`）会**被当成引用** ⇒ ⇒ 那种情况请写 `selector`。
fn guess_target(raw: &str) -> Target {
    const CSS_MARKS: [char; 16] = [
        '#', '.', '[', ']', '(', ')', ':', '>', '+', '~', '*', '=', '^', '$', '"', '\'',
    ];
    if raw.chars().any(|c| CSS_MARKS.contains(&c)) || raw.chars().any(char::is_whitespace) {
        Target::Selector(raw.to_string())
    } else {
        Target::Ref(raw.to_string())
    }
}

/// `fields` 必须是**数组** —— 引擎逐字回「Error: Missing fields array」。
fn parse_fields(value: Option<&Value>) -> Result<Option<Vec<FieldSpec>>, ToolError> {
    let Some(v) = value else {
        return Ok(None);
    };
    let arr = v.as_array().ok_or_else(|| {
        ToolError::InvalidParameters(
            "'fields' must be an ARRAY of {ref|selector, value, type} — the engine rejects a map"
                .into(),
        )
    })?;
    let mut out = Vec::with_capacity(arr.len());
    for item in arr {
        let target = parse_item_target(item).ok_or_else(|| {
            ToolError::InvalidParameters("every field needs a 'ref' or a 'selector'".into())
        })?;
        let value = item
            .get("value")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string();
        let kind = match item.get("type").and_then(|v| v.as_str()) {
            None => FieldKind::Text,
            Some(raw) => FieldKind::parse(raw).ok_or_else(|| {
                ToolError::InvalidParameters(format!("unsupported field type: {raw}"))
            })?,
        };
        out.push(FieldSpec {
            target,
            value,
            kind,
        });
    }
    Ok(Some(out))
}

fn parse_item_target(item: &Value) -> Option<Target> {
    if let Some(s) = item.get("selector").and_then(|v| v.as_str()) {
        return Some(Target::Selector(s.to_string()));
    }
    if let Some(r) = item.get("ref").and_then(|v| v.as_str()) {
        return Some(Target::Ref(r.to_string()));
    }
    None
}

/// 提交目标（会**提交** ⇒ ⇒ 最危险的一个）—— 引用与选择器**互斥**。
fn parse_optional_target(reference: Option<&str>, selector: Option<&str>) -> Option<Target> {
    if let Some(s) = selector {
        return Some(Target::Selector(s.to_string()));
    }
    if let Some(r) = reference {
        return Some(Target::Ref(r.to_string()));
    }
    None
}

fn parse_direction(value: Option<&Value>) -> Result<Option<ScrollDirection>, ToolError> {
    let Some(raw) = value.and_then(|v| v.as_str()) else {
        return Ok(None);
    };
    ScrollDirection::parse(raw).map(Some).ok_or_else(|| {
        ToolError::InvalidParameters(format!("unsupported scroll direction: {raw}"))
    })
}

fn act_kind(value: Option<&Value>) -> Result<ActKind, ToolError> {
    match value.and_then(|v| v.as_str()).unwrap_or("click") {
        "click" => Ok(ActKind::Click),
        "fill" => Ok(ActKind::Fill),
        "type" => Ok(ActKind::Type),
        "press" => Ok(ActKind::Press),
        "select" => Ok(ActKind::Select),
        "scroll" => Ok(ActKind::Scroll),
        "form" => Ok(ActKind::Form),
        other => Err(ToolError::InvalidParameters(format!(
            "unsupported action: {other}"
        ))),
    }
}
