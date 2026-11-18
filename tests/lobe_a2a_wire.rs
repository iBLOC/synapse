//! The lobe A2A wire — one entry point, two request shapes.
//!
//! The telencephalon speaks the **v0.2 JSON-RPC envelope**; the lobe
//! skills were written against the **v0.1 flat shape**. The entry point
//! understands both, so neither peer has to change its wire to reach the
//! other.
//!
//! The tests live here — not in the source file — because the whole point
//! of this shape is **what actually arrives on the wire**: they talk to a
//! real loopback listener.

use std::sync::Arc;

use async_trait::async_trait;
use axum::Router;
use serde_json::{json, Value};

use synapse::a2a::card::AgentCard;
use synapse::lobe::{
    build_a2a_router, parse_send_request, SkillContext, SkillOutput, StatelessSkill,
    StatelessSkillRouter,
};

/// A skill that echoes what it got — so the test can see exactly what the
/// entry point handed over.
struct EchoSkill;

#[async_trait]
impl StatelessSkill for EchoSkill {
    async fn handle(&self, ctx: Box<dyn SkillContext>) -> Result<SkillOutput, String> {
        let extra = ctx.extra().cloned().unwrap_or_default();
        Ok(SkillOutput {
            updated_history_summary: None,
            payload: json!({
                "user_id": ctx.user_id(),
                "extra": extra,
            }),
            audio_url: None,
        })
    }

    fn skill_id(&self) -> &'static str {
        "echo"
    }
}

fn router() -> Router {
    let mut skills = StatelessSkillRouter::new();
    skills.register(Arc::new(EchoSkill));
    build_a2a_router(
        AgentCard::new("echo-lobe", "test", "http://127.0.0.1:1", "0.1.0"),
        Arc::new(skills),
    )
}

/// The envelope the telencephalon really sends (2026-11-18 取证)：
/// the metadata map is packed behind `metadata.name`.
fn telencephalon_envelope() -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": "rpc-1",
        "method": "tasks/send",
        "params": {
            "id": "task-1",
            "message": {
                "role": "user",
                "parts": [
                    {"type": "Text", "text": "roster"},
                    {"type": "File", "mime_type": "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet", "name": "roster.xlsx", "bytes": "AAECAw=="}
                ],
                "metadata": {
                    "name": "__telencephalon_meta_v1__{\"skill\":\"echo\",\"user_id\":\"u-1\"}"
                }
            }
        }
    })
}

/// ⚠️ **这一格原来是最容易「看着像成功」的地方** —— JSON-RPC 信封到了
/// 只认扁平的入口 ⇒ ⇒ skill 读不到（4001）＋ ctx 是空的 ⇒ ⇒ 报
/// 「missing field user_id」⇒ ⇒ 看着像**对面叶区坏了**。
#[test]
fn the_jsonrpc_envelope_is_understood() {
    let req = parse_send_request(&telencephalon_envelope()).expect("the envelope must parse");
    assert_eq!(req.skill, "echo");
    assert_eq!(req.task_id, "task-1");
    assert!(req.jsonrpc, "the envelope must be recognised as JSON-RPC");
    assert_eq!(req.rpc_id, json!("rpc-1"));
    assert_eq!(req.ctx["user_id"], json!("u-1"));
    assert_eq!(req.ctx["metadata"]["skill"], json!("echo"));
    let parts = req.ctx["parts"].as_array().expect("the parts must survive");
    assert_eq!(parts.len(), 2);
    assert_eq!(parts[1]["name"], json!("roster.xlsx"));
    assert_eq!(parts[1]["bytes"], json!("AAECAw=="));
}

/// 旧的扁平形状**不许被这次改动弄坏**。
#[test]
fn a_flat_request_is_still_understood() {
    let body = json!({"skill": "echo", "message": {"user_id": "u-2", "hello": "world"}});
    let req = parse_send_request(&body).expect("the flat shape must parse");
    assert_eq!(req.skill, "echo");
    assert_eq!(req.ctx["user_id"], json!("u-2"));
    assert!(!req.jsonrpc, "the flat shape must not be answered with an envelope");
}

/// 没有 skill ⇒ 必须**报错**（不许猜一个）。
#[test]
fn a_request_without_a_skill_is_rejected() {
    let body = json!({"message": {"user_id": "u-3"}});
    let err = parse_send_request(&body).expect_err("a request without a skill must fail");
    assert_eq!(err.code, 4001);
}

/// 没有身份 ⇒ 必须**报错**（不许编一个 user_id ⇒ 那会把所有人的记忆
/// 混进一个桶）。
#[test]
fn a_request_without_an_identity_is_rejected() {
    let body = json!({"skill": "echo", "message": {"hello": "world"}});
    let err = parse_send_request(&body).expect_err("a request without an identity must fail");
    assert_eq!(err.code, 4002);
}

async fn spawn_lobe() -> String {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        let _ = axum::serve(listener, router()).await;
    });
    format!("http://{addr}")
}

/// ⚠️ **承载级实证** —— 真在回环上发一个 JSON-RPC 信封 ⇒ ⇒ skill 真跑了
/// ⇒ ⇒ 附件真到了 ⇒ ⇒ 身份真注入了。
#[tokio::test]
async fn the_jsonrpc_envelope_reaches_the_skill_with_its_attachment() {
    let base = spawn_lobe().await;
    let resp = reqwest::Client::new()
        .post(format!("{base}/api/a2a"))
        .json(&telencephalon_envelope())
        .send()
        .await
        .unwrap();
    assert!(resp.status().is_success(), "the leaf must answer");
    let out: Value = resp.json().await.unwrap();
    let message = out["result"]["task"]["status"]["message"]
        .as_str()
        .expect("the status message must be a string (clients read it as one)");
    assert!(message.contains("echo"), "the skill did not run: {message}");
    assert!(message.contains("roster.xlsx"), "the attachment did not survive: {message}");
    assert!(message.contains("u-1"), "the identity was not injected: {message}");
}

/// 旧的扁平形状 ⇒ 回**裸 payload**（老调用方是这么读的）。
#[tokio::test]
async fn the_flat_shape_still_answers_with_the_raw_payload() {
    let base = spawn_lobe().await;
    let body = json!({"skill": "echo", "message": {"user_id": "u-4"}});
    let resp = reqwest::Client::new()
        .post(format!("{base}/a2a/tasks/send"))
        .json(&body)
        .send()
        .await
        .unwrap();
    assert!(resp.status().is_success());
    let out: Value = resp.json().await.unwrap();
    assert_eq!(out["user_id"], json!("u-4"));
}

/// 卡片公布的是**基址** ⇒ ⇒ 基址也得能收 POST（否则注册了卡片、端脑
/// 一发就 404）。
#[tokio::test]
async fn the_base_url_of_the_card_also_answers() {
    let base = spawn_lobe().await;
    let body = json!({"skill": "echo", "message": {"user_id": "u-5"}});
    let resp = reqwest::Client::new()
        .post(format!("{base}/"))
        .json(&body)
        .send()
        .await
        .unwrap();
    assert!(resp.status().is_success(), "the card base URL must accept the send");
}

/// ⚠️ **「失败长得像成功」的那一格** —— 未知 skill 必须**报错**，
/// 不许回一个空回执（那会让端脑以为「跑通了，什么都没发生」）。
#[tokio::test]
async fn an_unknown_skill_is_reported_loudly() {
    let base = spawn_lobe().await;
    let body = json!({
        "jsonrpc": "2.0",
        "id": "rpc-2",
        "method": "tasks/send",
        "params": {
            "id": "task-2",
            "message": {
                "role": "user",
                "parts": [],
                "metadata": {"name": "__telencephalon_meta_v1__{\"skill\":\"nope\",\"user_id\":\"u-6\"}"}
            }
        }
    });
    let resp = reqwest::Client::new()
        .post(format!("{base}/api/a2a"))
        .json(&body)
        .send()
        .await
        .unwrap();
    let out: Value = resp.json().await.unwrap();
    assert!(
        out.get("error").is_some(),
        "an unknown skill must be reported, not silently accepted: {out}"
    );
}