//! 判据 —— 这一层的行为必须钉死。
//!
//! ⚠️ **负控**：假引擎复刻「引擎缺能力／引擎报错」两种形状 ⇒ ⇒
//! 同一支探针里就能做「修前 vs 修后」的对照。

use std::sync::{Arc, Mutex};

// ⚠️ **假引擎那几条用的 URL 是「字面公网 IP」**，不是域名 ——
// ⇒ ⇒ 判据就**与这台机器的解析器无关**（fake-IP 机器会把 example.com 映到
// benchmark 段 ⇒ ⇒ 用域名的话这条判据在 CI 上绿、在本机红 ⇒ ⇒ **机器依赖的
// 判据就是假判据**）。⇒ ⇒ 域名相关的行为另有专门一条（见
// `the_fakeip_probe_is_measured_not_assumed`）。
const FAKE_URL: &str = "http://93.184.216.34/";

use serde_json::{json, Value};

use super::engine::{
    obscura_args, obscura_tool, FieldKind, FieldSpec, ScrollDirection, Target, WebAction, WebEngine,
    WebError,
};
use super::policy::{check_url, detect_fakeip_resolver};
use super::server::mcp_server;
use super::{
    truncate_bytes, WebActTool, WebFetchTool, WebPolicy, WebReadTool, WebSession,
};
use crate::mcp::server::McpServer;
use crate::tools::{Tool, ToolError};

/// 假引擎：能力清单可配、返回值可配、动作会被记下来。
#[derive(Debug, Default)]
struct FakeEngine {
    caps: Vec<String>,
    body: String,
    fail: bool,
    actions: Mutex<Vec<String>>,
}

impl FakeEngine {
    fn with_cap(cap: &str) -> Self {
        Self {
            caps: vec![cap.to_string()],
            body: "ENGINE BODY".to_string(),
            fail: false,
            actions: Mutex::new(Vec::new()),
        }
    }
}

#[async_trait::async_trait]
impl WebEngine for FakeEngine {
    async fn capabilities(&self) -> Result<Vec<String>, WebError> {
        Ok(self.caps.clone())
    }

    fn tool_for(&self, _action: &WebAction) -> Option<String> {
        Some("fake_tool".to_string())
    }

    async fn invoke(&self, action: WebAction) -> Result<String, WebError> {
        self.actions.lock().unwrap().push(format!("{action:?}"));
        if self.fail {
            return Err(WebError::Engine("engine said no".into()));
        }
        Ok(self.body.clone())
    }
}

fn session(engine: FakeEngine, policy: WebPolicy) -> Arc<WebSession> {
    Arc::new(WebSession::new(Arc::new(engine), policy))
}

fn session_with(engine: FakeEngine) -> Arc<WebSession> {
    session(engine, WebPolicy::default())
}

#[tokio::test]
async fn web_fetch_returns_the_engine_content() {
    let s = session_with(FakeEngine::with_cap("fake_tool"));
    let out = WebFetchTool::new(s.clone())
        .execute(json!({"url": FAKE_URL}))
        .await
        .unwrap();
    assert!(out.content.contains("ENGINE BODY"), "actual: {}", out.content);
    assert!(!out.is_error);
}

#[tokio::test]
async fn web_fetch_refuses_a_loopback_literal() {
    let s = session_with(FakeEngine::with_cap("fake_tool"));
    let err = WebFetchTool::new(s)
        .execute(json!({"url": "http://127.0.0.1:8080/"}))
        .await
        .unwrap_err();
    assert!(matches!(err, ToolError::NotAuthorized(_)), "actual: {err:?}");
}

#[tokio::test]
async fn web_fetch_refuses_the_cloud_metadata_literal() {
    // 引擎自己的逃生口会放开这个地址 ⇒ ⇒ 这一层必须**照拒**。
    let s = session_with(FakeEngine::with_cap("fake_tool"));
    let err = WebFetchTool::new(s)
        .execute(json!({"url": "http://169.254.169.254/latest/meta-data/"}))
        .await
        .unwrap_err();
    assert!(matches!(err, ToolError::NotAuthorized(_)), "actual: {err:?}");
}

#[tokio::test]
async fn web_fetch_refuses_a_private_literal() {
    let s = session_with(FakeEngine::with_cap("fake_tool"));
    let err = WebFetchTool::new(s)
        .execute(json!({"url": "http://10.0.0.5/internal"}))
        .await
        .unwrap_err();
    assert!(matches!(err, ToolError::NotAuthorized(_)), "actual: {err:?}");
}

#[tokio::test]
async fn web_fetch_refuses_a_non_http_scheme() {
    let s = session_with(FakeEngine::with_cap("fake_tool"));
    let err = WebFetchTool::new(s)
        .execute(json!({"url": "file:///etc/passwd"}))
        .await
        .unwrap_err();
    assert!(matches!(err, ToolError::NotAuthorized(_)), "actual: {err:?}");
}

#[tokio::test]
async fn web_fetch_refuses_the_localhost_name() {
    let s = session_with(FakeEngine::with_cap("fake_tool"));
    let err = WebFetchTool::new(s)
        .execute(json!({"url": "http://localhost:3000/"}))
        .await
        .unwrap_err();
    assert!(matches!(err, ToolError::NotAuthorized(_)), "actual: {err:?}");
}

#[tokio::test]
async fn a_missing_engine_capability_is_reported_not_hidden() {
    // 引擎的能力清单里**没有**这个工具 ⇒ ⇒ 报「缺能力」，不是静默成功。
    let mut engine = FakeEngine::with_cap("something_else");
    engine.caps = vec!["other_tool".to_string()];
    let s = session_with(engine);
    let err = WebFetchTool::new(s)
        .execute(json!({"url": FAKE_URL}))
        .await
        .unwrap_err();
    assert!(matches!(err, ToolError::ToolDisabled(_)), "actual: {err:?}");
}

#[tokio::test]
async fn an_engine_failure_is_an_error_not_an_empty_success() {
    let mut engine = FakeEngine::with_cap("fake_tool");
    engine.fail = true;
    let s = session_with(engine);
    let err = WebFetchTool::new(s)
        .execute(json!({"url": FAKE_URL}))
        .await
        .unwrap_err();
    assert!(matches!(err, ToolError::ExternalServiceError(_)), "actual: {err:?}");
}

#[tokio::test]
async fn output_is_truncated_with_a_marker() {
    let mut engine = FakeEngine::with_cap("fake_tool");
    engine.body = "x".repeat(500);
    let policy = WebPolicy {
        max_output_bytes: 64,
        ..WebPolicy::default()
    };
    let s = session(engine, policy);
    let out = WebFetchTool::new(s)
        .execute(json!({"url": FAKE_URL}))
        .await
        .unwrap();
    assert!(out.content.contains("<truncated"), "actual: {}", out.content);
    assert!(out.content.len() < 200, "actual len: {}", out.content.len());
}

#[tokio::test]
async fn truncating_never_splits_a_multibyte_character() {
    // 3 字节一个字的字符串，截在字中间 ⇒ ⇒ 不能 panic，也不能出乱码。
    let text = "中".repeat(100);
    let cut = truncate_bytes(&text, 10);
    assert!(cut.contains("<truncated"));
    assert!(cut.is_char_boundary(0));
}

#[tokio::test]
async fn web_act_is_refused_by_default() {
    let s = session_with(FakeEngine::with_cap("fake_tool"));
    let err = WebActTool::new(s)
        .execute(json!({"action": "click", "target": "e1-1"}))
        .await
        .unwrap_err();
    assert!(matches!(err, ToolError::NotAuthorized(_)), "actual: {err:?}");
}

#[tokio::test]
async fn web_act_runs_when_the_policy_allows_it() {
    let policy = WebPolicy {
        allow_interactive: true,
        ..WebPolicy::default()
    };
    let s = session(FakeEngine::with_cap("fake_tool"), policy);
    let out = WebActTool::new(s)
        .execute(json!({"action": "click", "target": "e1-1"}))
        .await
        .unwrap();
    assert!(out.content.contains("ENGINE BODY"));
}

#[tokio::test]
async fn web_act_is_marked_as_a_write() {
    let s = session_with(FakeEngine::with_cap("fake_tool"));
    let meta = WebActTool::new(s).metadata();
    assert!(!meta.read_only, "interactive tool must not be read-only");
}

#[tokio::test]
async fn web_read_reads_the_current_page() {
    let s = session_with(FakeEngine::with_cap("fake_tool"));
    let out = WebReadTool::new(s)
        .execute(json!({"mode": "snapshot"}))
        .await
        .unwrap();
    assert!(out.content.contains("ENGINE BODY"));
}

#[tokio::test]
async fn the_mcp_server_exposes_our_own_names() {
    let s = session_with(FakeEngine::with_cap("fake_tool"));
    let server = mcp_server(s);
    let tools = server.list_tools().await.unwrap();
    let names: Vec<&str> = tools.iter().map(|t| t.name.as_str()).collect();
    assert_eq!(names.len(), 3, "actual: {names:?}");
    assert!(names.iter().all(|n| n.starts_with("web_")), "actual: {names:?}");
    // 引擎那 38 个 `browser_*` 名字**一个都不许露出来**
    // ⇒ ⇒ 端脑那边裸名不会撞 Playwright MCP。
    assert!(
        !names.iter().any(|n| n.contains("browser")),
        "engine tool names leaked: {names:?}"
    );
}

#[tokio::test]
async fn a_tool_failure_comes_back_as_an_mcp_error_result() {
    let mut engine = FakeEngine::with_cap("fake_tool");
    engine.fail = true;
    let s = session_with(engine);
    let server = mcp_server(s);
    let result = server
        .call_tool("web_fetch", json!({"url": FAKE_URL}))
        .await
        .unwrap();
    assert!(result.is_error, "a failed call must report isError");
}

#[test]
fn the_benchmark_range_is_only_open_when_the_policy_says_so() {
    // fake-IP 探测开着 ⇒ benchmark 段放行
    assert!(check_url("http://198.18.0.1/", true).is_ok());
    // 严格 ⇒ 拒
    assert!(check_url("http://198.18.0.1/", false).is_err());
    // 但 benchmark 段放开**不等于**私网放开
    assert!(check_url("http://10.0.0.1/", true).is_err());
    assert!(check_url("http://127.0.0.1/", true).is_err());
    assert!(check_url("http://169.254.169.254/", true).is_err());
}

#[test]
fn the_fakeip_probe_is_measured_not_assumed() {
    // 这台机器是什么就是什么 —— 只断言「探测函数有结论」，不假设结论。
    let detected = detect_fakeip_resolver();
    if detected {
        // 探测说是 fake-IP ⇒ 那 benchmark 段必须能过门（否则一个站都打不开）
        assert!(check_url("http://198.18.0.1/", true).is_ok());
    } else {
        assert!(check_url("http://198.18.0.1/", false).is_err());
    }
}

/// 引擎够得着**回环**上的 MCP 端点 —— 即使这台机器开着系统代理。
///
/// ⚠️ 这条判据钉的是「引擎不许走代理」：Windows 的系统代理（Clash：
/// 127.0.0.1:7897）开着时，reqwest 默认会把回环请求**也**发给代理
/// ⇒ ⇒ 代理够不到那个端口 ⇒ ⇒ 回 502（空 body）。
/// ⇒ ⇒ 引擎的 client 必须 `no_proxy()`。
#[tokio::test]
async fn the_engine_reaches_a_local_mcp_endpoint_even_under_a_system_proxy() {
    // 起一个**本机**假 MCP server（回环 ⇒ 正是代理会插手的那种端点）
    let app = axum::Router::new().route(
        "/mcp",
        axum::routing::post(|body: String| async move {
            let parsed: serde_json::Value = serde_json::from_str(&body).unwrap_or(json!({}));
            let id = parsed.get("id").cloned().unwrap_or(json!(0));
            let method = parsed.get("method").and_then(|m| m.as_str()).unwrap_or("");
            let result = if method == "tools/list" {
                // 报**引擎自己的**工具名 —— 这一层就是靠它做「运行时自省」
                json!({"tools": [{"name": "browser_navigate"}, {"name": "browser_markdown"}]})
            } else {
                json!({"content": [{"type": "text", "text": "LOCAL SERVER BODY"}]})
            };
            (
                axum::http::StatusCode::OK,
                axum::Json(json!({"jsonrpc": "2.0", "id": id, "result": result})),
            )
        }),
    );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        let _ = axum::serve(listener, app).await;
    });
    let endpoint = format!("http://{addr}/mcp");

    let engine = super::engine::ObscuraEngine::new(endpoint, None);
    let s = Arc::new(WebSession::new(Arc::new(engine), WebPolicy::default()));
    let out = WebFetchTool::new(s)
        .execute(json!({"url": FAKE_URL}))
        .await
        .unwrap();
    assert!(
        out.content.contains("LOCAL SERVER BODY"),
        "actual: {}",
        out.content
    );
}

/// 真链路：真 obscura 进程（环境变量给端点）。
///
/// ⚠️ 没有端点就**跳过并说明** —— 不假装绿。
#[tokio::test]
async fn the_real_engine_path_fetches_a_real_page() {
    let Ok(endpoint) = std::env::var("SYNAPSE_WEB_ENGINE_URL") else {
        eprintln!(
            "SKIP the_real_engine_path_fetches_a_real_page: SYNAPSE_WEB_ENGINE_URL unset \
             (start: obscura mcp --http --port 3111, with OBSCURA_ALLOW_PRIVATE_NETWORK=1 \
             when the host resolver is fake-IP)"
        );
        return;
    };
    let _guard = REAL_ENGINE_LOCK.lock().await;
    let engine = super::engine::ObscuraEngine::new(endpoint, None);
    let policy = WebPolicy::detect();
    let s = Arc::new(WebSession::new(Arc::new(engine), policy));
    // ⚠️ 真链路用**真域名**（这里是真的 example.com）—— 字面 IP 过不了 TLS。
    let out = WebFetchTool::new(s)
        .execute(json!({"url": "https://example.com/"}))
        .await
        .unwrap();
    assert!(
        out.content.contains("Example Domain"),
        "actual: {}",
        out.content
    );
}

// ── 这一刀的判据：动作 → 引擎实参，**逐字**对过真 schema ─────────────────
//
// ⚠️ 真引擎（obscura 0.2.7）当场回过的错误，逐字抄在这儿当**对照**：
//   type  传 value  ⇒ 「Error: Missing text parameter」
//   select 传 ref   ⇒ 「Error: Missing selector parameter」
//   scroll 只传 selector ⇒ 「Scrolled element into view」（**静默做错事**）
//   form  传对象    ⇒ 「Error: Missing fields array」

fn act(kind: super::engine::ActKind) -> WebAction {
    WebAction::Act {
        kind,
        target: None,
        value: None,
        key: None,
        fields: None,
        direction: None,
        amount: None,
        submit: None,
    }
}

#[test]
fn the_type_action_sends_text_not_value() {
    let mut a = act(super::engine::ActKind::Type);
    if let WebAction::Act { target, value, .. } = &mut a {
        *target = Some(Target::Ref("e4-1".into()));
        *value = Some("hello".into());
    }
    assert_eq!(obscura_tool(&a), Some("browser_type"));
    let args = obscura_args(&a).unwrap();
    assert_eq!(args.get("text").and_then(|v| v.as_str()), Some("hello"));
    assert!(
        args.get("value").is_none(),
        "the engine's parameter is 'text' — 'value' would be rejected: {args}"
    );
}

#[test]
fn the_fill_action_sends_value() {
    let mut a = act(super::engine::ActKind::Fill);
    if let WebAction::Act { target, value, .. } = &mut a {
        *target = Some(Target::Selector("#q".into()));
        *value = Some("shadow".into());
    }
    assert_eq!(obscura_tool(&a), Some("browser_fill"));
    let args = obscura_args(&a).unwrap();
    assert_eq!(args.get("value").and_then(|v| v.as_str()), Some("shadow"));
    assert_eq!(args.get("selector").and_then(|v| v.as_str()), Some("#q"));
    assert!(args.get("ref").is_none(), "a selector must not also send 'ref': {args}");
}

#[test]
fn the_select_action_refuses_a_ref_and_takes_a_selector() {
    let mut a = act(super::engine::ActKind::Select);
    if let WebAction::Act { target, value, .. } = &mut a {
        *target = Some(Target::Ref("e4-1".into()));
        *value = Some("x".into());
    }
    let err = obscura_args(&a).unwrap_err();
    assert!(
        matches!(&err, WebError::BadArgument(m) if m.contains("selector")),
        "a ref must be refused in the truth: {err:?}"
    );
    if let WebAction::Act { target, .. } = &mut a {
        *target = Some(Target::Selector("select#lang".into()));
    }
    let args = obscura_args(&a).unwrap();
    assert_eq!(args.get("selector").and_then(|v| v.as_str()), Some("select#lang"));
    assert_eq!(args.get("value").and_then(|v| v.as_str()), Some("x"));
}

#[test]
fn the_press_action_refuses_a_ref() {
    let mut a = act(super::engine::ActKind::Press);
    if let WebAction::Act { target, key, .. } = &mut a {
        *target = Some(Target::Ref("e4-1".into()));
        *key = Some("Enter".into());
    }
    let err = obscura_args(&a).unwrap_err();
    assert!(
        matches!(&err, WebError::BadArgument(m) if m.contains("selector")),
        "actual: {err:?}"
    );
}

#[test]
fn the_scroll_action_carries_a_direction() {
    let mut a = act(super::engine::ActKind::Scroll);
    // 没有方向 ⇒ 引擎会做「把元素滚进视野」⇒ ⇒ 我们**先**拒掉。
    assert!(
        obscura_args(&a).is_err(),
        "scroll without a direction must be refused, not silently turned into 'scroll into view'"
    );
    if let WebAction::Act { direction, amount, .. } = &mut a {
        *direction = Some(ScrollDirection::Bottom);
        *amount = Some(600);
    }
    let args = obscura_args(&a).unwrap();
    assert_eq!(args.get("direction").and_then(|v| v.as_str()), Some("bottom"));
    assert_eq!(args.get("amount").and_then(|v| v.as_u64()), Some(600));
}

#[test]
fn the_form_action_sends_an_array_of_fields() {
    let mut a = act(super::engine::ActKind::Form);
    if let WebAction::Act { fields, submit, .. } = &mut a {
        *fields = Some(vec![
            FieldSpec {
                target: Target::Selector("#name".into()),
                value: "ada".into(),
                kind: FieldKind::Text,
            },
            FieldSpec {
                target: Target::Ref("e5-2".into()),
                value: String::new(),
                kind: FieldKind::Check,
            },
        ]);
        *submit = Some(Target::Selector("#go".into()));
    }
    let args = obscura_args(&a).unwrap();
    let fields = args
        .get("fields")
        .and_then(|v| v.as_array())
        .unwrap_or_else(|| panic!("fields must be an array: {args}"));
    assert_eq!(fields.len(), 2, "actual: {fields:?}");
    assert_eq!(fields[0].get("value").and_then(|v| v.as_str()), Some("ada"));
    assert_eq!(fields[0].get("selector").and_then(|v| v.as_str()), Some("#name"));
    assert_eq!(fields[1].get("ref").and_then(|v| v.as_str()), Some("e5-2"));
    assert_eq!(fields[1].get("type").and_then(|v| v.as_str()), Some("check"));
    assert_eq!(
        args.get("submit_selector").and_then(|v| v.as_str()),
        Some("#go")
    );
}

#[tokio::test]
async fn web_act_rejects_fields_given_as_a_map() {
    // 旧形状是对象 ⇒ ⇒ 引擎逐字回「Error: Missing fields array」
    // ⇒ ⇒ 门面**先**说清「要数组」，别让模型去撞引擎的错误。
    let policy = WebPolicy {
        allow_interactive: true,
        ..WebPolicy::default()
    };
    let s = session(FakeEngine::with_cap("fake_tool"), policy);
    let err = WebActTool::new(s)
        .execute(json!({"action": "form", "fields": {"name": "ada"}}))
        .await
        .unwrap_err();
    assert!(
        matches!(err, ToolError::InvalidParameters(_)),
        "actual: {err:?}"
    );
}

#[tokio::test]
async fn web_act_scroll_without_a_direction_is_refused() {
    let policy = WebPolicy {
        allow_interactive: true,
        ..WebPolicy::default()
    };
    let s = session(FakeEngine::with_cap("fake_tool"), policy);
    let err = WebActTool::new(s)
        .execute(json!({"action": "scroll"}))
        .await
        .unwrap_err();
    assert!(
        matches!(err, ToolError::InvalidParameters(_)),
        "actual: {err:?}"
    );
}

/// 引擎层知道「obscura 的 `select_option` 只收 selector」⇒ ⇒
/// 门面把它**如实**报成「参数不对」，**不是**「引擎挂了」⇒ ⇒ 失败要长得像失败。
#[tokio::test]
async fn web_act_select_with_a_ref_is_reported_as_a_parameter_error() {
    let (endpoint, _calls) = recording_engine().await;
    let engine = super::engine::ObscuraEngine::new(endpoint, None);
    let policy = WebPolicy {
        allow_interactive: true,
        ..WebPolicy::default()
    };
    let s = Arc::new(WebSession::new(Arc::new(engine), policy));
    let err = WebActTool::new(s)
        .execute(json!({"action": "select", "ref": "e4-1", "value": "x"}))
        .await
        .unwrap_err();
    assert!(
        matches!(err, ToolError::InvalidParameters(_)),
        "actual: {err:?}"
    );
}

/// **线级**判据：引擎**真的收到**的参数名，逐字对得上。
///
/// ⚠️ 这条比纯函数那条更硬：它过的是「门面 → 引擎 → 线上 JSON」整条链。
#[tokio::test]
async fn the_engine_receives_the_exact_parameter_names() {
    let (endpoint, calls) = recording_engine().await;
    let engine = super::engine::ObscuraEngine::new(endpoint, None);
    let policy = WebPolicy {
        allow_interactive: true,
        ..WebPolicy::default()
    };
    let s = Arc::new(WebSession::new(Arc::new(engine), policy));
    let tool = WebActTool::new(s.clone());

    tool.execute(json!({"action": "type", "ref": "e4-1", "value": "hello"}))
        .await
        .unwrap();
    tool.execute(json!({"action": "scroll", "direction": "bottom", "amount": 600}))
        .await
        .unwrap();
    tool.execute(json!({"action": "form", "fields": [{"selector": "#name", "value": "ada"}]}))
        .await
        .unwrap();

    let calls = calls.lock().unwrap();
    assert_eq!(calls.len(), 3, "actual: {calls:?}");

    // type ⇒ 引擎收到的是 `text`（**不是** `value`）
    let (name, args) = &calls[0];
    assert_eq!(name, "browser_type", "actual: {calls:?}");
    assert_eq!(args.get("text").and_then(|v| v.as_str()), Some("hello"));
    assert!(args.get("value").is_none(), "actual: {args}");

    // scroll ⇒ 引擎收到 `direction`
    let (name, args) = &calls[1];
    assert_eq!(name, "browser_scroll", "actual: {calls:?}");
    assert_eq!(args.get("direction").and_then(|v| v.as_str()), Some("bottom"));

    // form ⇒ 引擎收到**数组**
    let (name, args) = &calls[2];
    assert_eq!(name, "browser_fill_form", "actual: {calls:?}");
    assert!(
        args.get("fields").and_then(|v| v.as_array()).is_some(),
        "fields must reach the engine as an array: {args}"
    );
}

/// 起一个**会记下实参**的假 MCP 引擎（回环上 ⇒ 正是代理会插手的那种端点）。
async fn recording_engine() -> (String, Arc<Mutex<Vec<(String, Value)>>>) {
    let calls: Arc<Mutex<Vec<(String, Value)>>> = Arc::new(Mutex::new(Vec::new()));
    let sink = calls.clone();
    let app = axum::Router::new().route(
        "/mcp",
        axum::routing::post(move |body: String| {
            let sink = sink.clone();
            async move {
                let parsed: Value = serde_json::from_str(&body).unwrap_or(json!({}));
                let id = parsed.get("id").cloned().unwrap_or(json!(0));
                let method = parsed
                    .get("method")
                    .and_then(|m| m.as_str())
                    .unwrap_or("")
                    .to_string();
                let result = if method == "tools/list" {
                    json!({"tools": [
                        {"name": "browser_navigate"}, {"name": "browser_markdown"},
                        {"name": "browser_snapshot"}, {"name": "browser_click"},
                        {"name": "browser_fill"}, {"name": "browser_type"},
                        {"name": "browser_press_key"}, {"name": "browser_select_option"},
                        {"name": "browser_scroll"}, {"name": "browser_fill_form"},
                        {"name": "browser_extract"}, {"name": "browser_links"},
                        {"name": "browser_console_messages"}, {"name": "browser_network_requests"}
                    ]})
                } else {
                    let name = parsed
                        .get("params")
                        .and_then(|p| p.get("name"))
                        .and_then(|n| n.as_str())
                        .unwrap_or("")
                        .to_string();
                    let arguments = parsed
                        .get("params")
                        .and_then(|p| p.get("arguments"))
                        .cloned()
                        .unwrap_or(json!({}));
                    sink.lock().unwrap().push((name, arguments));
                    json!({"content": [{"type": "text", "text": "RECORDED"}]})
                };
                (
                    axum::http::StatusCode::OK,
                    axum::Json(json!({"jsonrpc": "2.0", "id": id, "result": result})),
                )
            }
        }),
    );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        let _ = axum::serve(listener, app).await;
    });
    (format!("http://{addr}/mcp"), calls)
}

/// 真链路判据**共用一把锁** —— ⚠️ 真引擎是**一个浏览器一个页面**
/// ⇒ ⇒ Rust 的测试默认**并发**跑 ⇒ ⇒ 两条真链路判据会互相踩
/// （一条在 navigate，另一条在点链接 ⇒ ⇒ 读到的是对方的页面 ⇒ ⇒ flaky）。
static REAL_ENGINE_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

/// 真链路：真 obscura 进程（环境变量给端点）—— **交互动作**。
///
/// ⚠️ 这一格是「点一个链接」⇒ ⇒ 没有副作用（只是 GET 一次公开页）。
/// ⚠️ ref 是**按导航纪元编号**的 ⇒ ⇒ 必须刚 snapshot 就用。
#[tokio::test]
async fn the_real_engine_path_clicks_a_link_and_lands_on_the_target() {
    let Ok(endpoint) = std::env::var("SYNAPSE_WEB_ENGINE_URL") else {
        eprintln!(
            "SKIP the_real_engine_path_clicks_a_link_and_lands_on_the_target:              SYNAPSE_WEB_ENGINE_URL unset"
        );
        return;
    };
    let _guard = REAL_ENGINE_LOCK.lock().await;
    let engine = super::engine::ObscuraEngine::new(endpoint, None);
    let policy = WebPolicy {
        allow_interactive: true,
        ..WebPolicy::detect()
    };
    let s = Arc::new(WebSession::new(Arc::new(engine), policy));
    WebFetchTool::new(s.clone())
        .execute(json!({"url": "https://example.com/"}))
        .await
        .unwrap();
    let snap = WebReadTool::new(s.clone())
        .execute(json!({"mode": "snapshot"}))
        .await
        .unwrap();
    let reference = extract_ref(&snap.content)
        .unwrap_or_else(|| panic!("no ref in the snapshot: {}", snap.content));
    WebActTool::new(s.clone())
        .execute(json!({"action": "click", "ref": reference}))
        .await
        .unwrap();
    // ⚠️ 落点用 **snapshot** 的 URL 判 ⇒ ⇒ 比「正文里有没有 iana」精确。
    let after = WebReadTool::new(s)
        .execute(json!({"mode": "snapshot"}))
        .await
        .unwrap();
    assert!(
        !after.content.contains("This domain is for use in documentation examples"),
        "still on example.com: {}",
        after.content
    );
    assert!(
        after.content.contains("iana.org"),
        "did not land on the iana.org page: {}",
        after.content
    );
}

/// 从 snapshot 文本里抠出**当前这一次页面加载**的 ref（形如 `e4-1`）。
fn extract_ref(snapshot: &str) -> Option<String> {
    for line in snapshot.lines() {
        let trimmed = line.trim();
        let Some(rest) = trimmed.strip_prefix("ref=") else {
            continue;
        };
        let id: String = rest
            .chars()
            .take_while(|c| c.is_ascii_alphanumeric() || *c == '-')
            .collect();
        if !id.is_empty() {
            return Some(id);
        }
    }
    None
}

// ── 分发链：引擎发行包的摘要 ───────────────────────────────────────────

#[test]
fn the_pinned_digest_table_is_well_formed() {
    use super::engine_binary::OFFICIAL_DIGESTS;
    assert_eq!(
        OFFICIAL_DIGESTS.len(),
        20,
        "the official v0.2.7 release ships 20 assets"
    );
    let mut assets = std::collections::HashSet::new();
    let mut digests = std::collections::HashSet::new();
    for (asset, digest) in OFFICIAL_DIGESTS {
        assert!(asset.starts_with("obscura-"), "bad asset name: {asset}");
        assert_eq!(digest.len(), 64, "sha256 is 64 hex chars: {asset}");
        assert!(
            digest
                .chars()
                .all(|c| c.is_ascii_digit() || ('a'..='f').contains(&c)),
            "not lowercase hex: {asset} {digest}"
        );
        assert!(assets.insert(*asset), "duplicate asset: {asset}");
        assert!(digests.insert(*digest), "duplicate digest: {asset}");
    }
}

#[test]
fn the_sha256_helper_matches_the_known_vector() {
    use super::engine_binary::sha256_file;
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("abc.bin");
    std::fs::write(&path, b"abc").unwrap();
    assert_eq!(
        sha256_file(&path).unwrap(),
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
}

#[test]
fn a_mismatched_artifact_is_refused() {
    use super::engine_binary::{verify_artifact, DigestError};
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("obscura-x86_64-windows.zip");
    std::fs::write(&path, b"not the official artifact").unwrap();
    let err = verify_artifact(&path, "obscura-x86_64-windows.zip").unwrap_err();
    assert!(
        matches!(err, DigestError::Mismatch { .. }),
        "actual: {err:?}"
    );
}

#[test]
fn a_renamed_artifact_is_still_matched_to_its_official_asset() {
    use super::engine_binary::guess_asset;
    // 官方名字原样
    assert_eq!(
        guess_asset("obscura-x86_64-windows.zip"),
        Some("obscura-x86_64-windows.zip")
    );
    // ⚠️ 实测：本机那份就被改过名（`obscura-windows.zip`）⇒ ⇒ 猜得到才核得上
    assert_eq!(
        guess_asset("obscura-windows.zip"),
        Some("obscura-x86_64-windows.zip")
    );
    assert_eq!(
        guess_asset("obscura-aarch64-linux-no-render-stealth.tar.gz"),
        Some("obscura-aarch64-linux-no-render-stealth.tar.gz")
    );
    assert_eq!(
        guess_asset("obscura-linux-x64-stealth.tar.gz"),
        Some("obscura-x86_64-linux-stealth.tar.gz")
    );
    // 认不得就**不猜**（比错资产 ＝ 比了个寂寞）
    assert_eq!(guess_asset("random.bin"), None);
}

#[test]
fn an_unknown_asset_name_is_refused() {
    use super::engine_binary::{verify_artifact, DigestError};
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("obscura-from-somewhere-else.zip");
    std::fs::write(&path, b"whatever").unwrap();
    let err = verify_artifact(&path, "obscura-from-somewhere-else.zip").unwrap_err();
    assert!(
        matches!(err, DigestError::UnknownAsset(_, _)),
        "actual: {err:?}"
    );
}

// ── 清单机制：「换版本不必改代码」那一族 ──────────────────────────────

fn manifest_with(version: &str, assets: &[(&str, &str)]) -> super::engine_binary::DigestManifest {
    super::engine_binary::DigestManifest {
        version: version.to_string(),
        source_url: "https://api.github.com/repos/h4ckf0r0day/obscura/releases".to_string(),
        retrieved_at: "2026-11-16T00:00:00Z".to_string(),
        assets: assets
            .iter()
            .map(|(name, sha256)| super::engine_binary::AssetDigest {
                name: (*name).to_string(),
                sha256: (*sha256).to_string(),
            })
            .collect(),
    }
}

#[test]
fn the_builtin_anchor_can_be_exported_as_a_manifest() {
    use super::engine_binary::{is_sha256_hex, DigestManifest, ENGINE_VERSION, OFFICIAL_DIGESTS};
    let m = DigestManifest::from_builtin();
    assert_eq!(m.version, ENGINE_VERSION);
    assert_eq!(m.assets.len(), OFFICIAL_DIGESTS.len());
    for a in &m.assets {
        assert!(is_sha256_hex(&a.sha256), "bad digest in the anchor: {a:?}");
    }
}

#[test]
fn a_manifest_round_trips_through_json() {
    use super::engine_binary::DigestManifest;
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("manifest.json");
    let original = DigestManifest::from_builtin();
    std::fs::write(&path, original.to_json().as_bytes()).unwrap();
    let loaded = DigestManifest::load(&path).unwrap();
    assert_eq!(loaded.version, original.version);
    assert_eq!(loaded.assets.len(), original.assets.len());
    for a in &original.assets {
        assert_eq!(
            loaded.digest_of(&a.name),
            Some(a.sha256.as_str()),
            "actual: {:?}",
            loaded.digest_of(&a.name)
        );
    }
}

#[test]
fn the_manifest_and_the_anchor_agree_for_the_pinned_version() {
    use super::engine_binary::{
        official_digest, resolve_expected_digest, DigestManifest, DigestSource, ENGINE_VERSION,
    };
    let m = DigestManifest::from_builtin();
    let asset = "obscura-x86_64-windows.zip";
    let (digest, source) = resolve_expected_digest(asset, Some(&m)).unwrap();
    assert_eq!(source, DigestSource::Anchor, "the anchor rules for its own version");
    assert_eq!(digest, official_digest(asset).unwrap());
    assert_eq!(m.version, ENGINE_VERSION);
}

#[test]
fn a_manifest_that_disagrees_with_the_anchor_for_the_same_version_is_refused() {
    use super::engine_binary::{
        resolve_expected_digest, DigestError, DigestManifest, ENGINE_VERSION,
    };
    // ⚠️ 同一版、两份摘要不一样 ⇒ ⇒ 有一方被改过 ⇒ ⇒ 响亮报错，不静默取其一。
    let m = manifest_with(
        ENGINE_VERSION,
        &[("obscura-x86_64-windows.zip", &"0".repeat(64))],
    );
    let err = resolve_expected_digest("obscura-x86_64-windows.zip", Some(&m)).unwrap_err();
    assert!(matches!(err, DigestError::ManifestConflict { .. }), "actual: {err:?}");
}

#[test]
fn the_manifest_takes_over_for_another_version() {
    use super::engine_binary::{resolve_expected_digest, DigestSource};
    // 「升级」那一格：清单说的是**另一版** ⇒ ⇒ 锚说的是另一版 ⇒ ⇒ 用清单。
    let other = "1".repeat(64);
    let m = manifest_with("0.2.6", &[("obscura-x86_64-windows.zip", &other)]);
    let (digest, source) = resolve_expected_digest("obscura-x86_64-windows.zip", Some(&m)).unwrap();
    assert_eq!(source, DigestSource::Manifest);
    assert_eq!(digest, other);
}

#[test]
fn the_manifest_takes_over_for_an_asset_the_anchor_never_heard_of() {
    use super::engine_binary::{resolve_expected_digest, DigestSource};
    let fresh = "2".repeat(64);
    let m = manifest_with("0.2.6", &[("obscura-x86_64-windows-v026.zip", &fresh)]);
    let (digest, source) =
        resolve_expected_digest("obscura-x86_64-windows-v026.zip", Some(&m)).unwrap();
    assert_eq!(source, DigestSource::Manifest);
    assert_eq!(digest, fresh);
}

#[test]
fn a_manifest_without_the_asset_leaves_the_anchor_in_charge() {
    use super::engine_binary::{resolve_expected_digest, DigestSource};
    let m = manifest_with("0.2.6", &[("obscura-aarch64-macos.tar.gz", &"3".repeat(64))]);
    let (digest, source) = resolve_expected_digest("obscura-x86_64-windows.zip", Some(&m)).unwrap();
    assert_eq!(source, DigestSource::Anchor);
    assert_eq!(
        digest,
        super::engine_binary::official_digest("obscura-x86_64-windows.zip").unwrap()
    );
}

#[test]
fn an_asset_unknown_to_both_sources_is_refused() {
    use super::engine_binary::{resolve_expected_digest, DigestError, DigestManifest};
    let m = DigestManifest::from_builtin();
    let err = resolve_expected_digest("obscura-from-somewhere-else.zip", Some(&m)).unwrap_err();
    assert!(matches!(err, DigestError::UnknownAsset(_, _)), "actual: {err:?}");
}

#[test]
fn a_broken_manifest_file_is_refused_loudly() {
    use super::engine_binary::{DigestError, DigestManifest};
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("manifest.json");
    std::fs::write(&path, b"{not json at all").unwrap();
    let err = DigestManifest::load(&path).unwrap_err();
    assert!(matches!(err, DigestError::ManifestParse { .. }), "actual: {err:?}");
}

#[test]
fn a_manifest_without_a_version_is_refused() {
    use super::engine_binary::{DigestError, DigestManifest};
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("manifest.json");
    std::fs::write(&path, br#"{"version":"","assets":[]}"#).unwrap();
    let err = DigestManifest::load(&path).unwrap_err();
    assert!(matches!(err, DigestError::ManifestParse { .. }), "actual: {err:?}");
}

#[test]
fn a_manifest_with_a_malformed_digest_is_refused() {
    use super::engine_binary::{DigestError, DigestManifest};
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("manifest.json");
    std::fs::write(
        &path,
        br#"{"version":"0.2.6","assets":[{"name":"obscura-x86_64-windows.zip","sha256":"not-a-digest"}]}"#,
    )
    .unwrap();
    let err = DigestManifest::load(&path).unwrap_err();
    assert!(
        matches!(err, DigestError::ManifestBadDigest { .. }),
        "actual: {err:?}"
    );
}

#[test]
fn verification_reports_which_source_the_digest_came_from() {
    // 承载级：文件 ⇒ ⇒ 摘要 ⇒ ⇒ 比对 ⇒ ⇒ **并说清这份摘要是谁给的**。
    use super::engine_binary::{sha256_file, verify_artifact_with, DigestSource};
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("obscura-x86_64-windows.zip");
    std::fs::write(&path, b"pretend artifact").unwrap();
    let digest = sha256_file(&path).unwrap();
    // ⚠️ 清单说的是**另一版** ⇒ ⇒ 清单接管（升级那一格）⇒ ⇒ 出处＝清单。
    let m = manifest_with("0.2.6", &[("obscura-x86_64-windows.zip", &digest)]);
    let verified = verify_artifact_with(&path, "obscura-x86_64-windows.zip", Some(&m)).unwrap();
    assert_eq!(verified.digest, digest);
    assert_eq!(verified.source, DigestSource::Manifest);
}

#[test]
fn verification_refuses_an_artifact_that_does_not_match_the_manifest() {
    use super::engine_binary::{verify_artifact_with, DigestError, DigestManifest};
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("obscura-x86_64-windows.zip");
    std::fs::write(&path, b"pretend artifact").unwrap();
    let m = DigestManifest::from_builtin();
    let err =
        verify_artifact_with(&path, "obscura-x86_64-windows.zip", Some(&m)).unwrap_err();
    assert!(matches!(err, DigestError::Mismatch { .. }), "actual: {err:?}");
}



