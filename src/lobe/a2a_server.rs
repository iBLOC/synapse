//! Build an axum router exposing the lobe A2A endpoints.
//!
//! Endpoints:
//!
//! - `GET /.well-known/agent.json` — the card.
//! - `POST /a2a/tasks/send` — the canonical A2A send endpoint.
//! - `POST /api/a2a` — the path the reference lobes register with the
//!   telencephalon (`http://<bind>/api/a2a`).
//! - `POST /` — the base URL the card advertises.
//! - `GET /a2a/tasks/{id}` / `POST /a2a/tasks/{id}/cancel` — stubs.
//!
//! ## Two wire shapes, one endpoint
//!
//! The telencephalon speaks the **v0.2 JSON-RPC envelope**
//! (`{jsonrpc, id, method, params:{id, message}}`); the lobe skills were
//! written against the **v0.1 flat shape** (`{skill, message}`).
//!
//! Accepting only one of them makes the other fail in a way that looks
//! like the peer is broken: the JSON-RPC envelope reaches the flat reader
//! as an empty context ⇒ ⇒ the skill name is missing (4001) and the
//! context fails to parse (`missing field user_id`).
//!
//! So the entry point parses **both** shapes into one [`SendRequest`]
//! before dispatching, and answers in the shape the caller used.
//!
//! ## The identity has to come from somewhere
//!
//! `JsonCtx` requires a `user_id`. The telencephalon carries it in the
//! metadata map it packs into `metadata.name`
//! (`__telencephalon_meta_v1__{...}`) ⇒ ⇒ the entry point decodes that
//! pack and injects `user_id` into the context. A request that carries
//! no identity at all is **rejected** (4002) rather than handed a
//! made-up one.
//!
//! ## Why the send endpoint is mounted on three paths
//!
//! The telencephalon POSTs the JSON-RPC envelope to the URL the lobe
//! **registered** — no path is appended ⇒ ⇒ that URL has to be an
//! endpoint that exists. The reference lobes register
//! `http://<bind>/api/a2a`, while the card advertises the base
//! `http://<bind>` ⇒ ⇒ both are mounted here, next to the canonical
//! `/a2a/tasks/send`.

use std::sync::Arc;

use axum::{routing::{get, post}, Json, Router};
use serde_json::{json, Map, Value};

use crate::a2a::card::AgentCard;
use crate::lobe::StatelessSkillRouter;

/// The prefix the telencephalon packs its metadata map under
/// (`metadata.name`).
const TELENCEPHALON_META_PREFIX: &str = "__telencephalon_meta_v1__";
/// The pre-rename spelling of the same channel.
const CORTEX_META_PREFIX: &str = "__cortex_meta_v1__";

/// A `tasks/send` request, in either wire shape.
#[derive(Debug, Clone)]
pub struct SendRequest {
    /// The task id (the JSON-RPC `params.id`; the flat `id` as fallback).
    pub task_id: String,
    /// Which skill to run.
    pub skill: String,
    /// The context handed to the skill.
    pub ctx: Value,
    /// Whether the caller spoke the JSON-RPC envelope.
    pub jsonrpc: bool,
    /// The JSON-RPC `id` to echo back.
    pub rpc_id: Value,
}

/// Why a request could not be understood.
#[derive(Debug, Clone)]
pub struct SendError {
    /// The error code (4001 = no skill, 4002 = no identity).
    pub code: i32,
    /// What the caller has to fix.
    pub message: String,
}

/// Decode the metadata map the telencephalon packs into `metadata.name`.
///
/// The pack is a JSON object behind a magic prefix. A peer that puts the
/// structured metadata straight into `name` (no prefix) is accepted too.
fn decode_packed_metadata(name: &str) -> Option<Value> {
    if let Some(payload) = name
        .strip_prefix(TELENCEPHALON_META_PREFIX)
        .or_else(|| name.strip_prefix(CORTEX_META_PREFIX))
    {
        return serde_json::from_str::<Value>(payload).ok();
    }
    let parsed = serde_json::from_str::<Value>(name).ok()?;
    if parsed.is_object() { Some(parsed) } else { None }
}

/// The `message` object of the request — JSON-RPC first, then flat.
fn request_message(body: &Value) -> Option<&Value> {
    body.get("params")
        .and_then(|p| p.get("message"))
        .or_else(|| body.get("message"))
        .filter(|m| m.is_object())
}

/// The structured metadata of the request.
///
/// Sources, in order (a later one does not overwrite an earlier one):
/// the top-level `metadata`, `params.metadata`, the message metadata, and
/// the pack behind `metadata.name`.
fn request_metadata(body: &Value, message: Option<&Value>) -> Map<String, Value> {
    let mut merged = Map::new();
    for source in [
        body.get("metadata"),
        body.get("params").and_then(|p| p.get("metadata")),
        message.and_then(|m| m.get("metadata")),
    ] {
        if let Some(obj) = source.and_then(|v| v.as_object()) {
            for (k, v) in obj {
                merged.entry(k.clone()).or_insert_with(|| v.clone());
            }
        }
    }
    if let Some(name) = merged.get("name").and_then(|v| v.as_str()) {
        if let Some(packed) = decode_packed_metadata(name).and_then(|v| v.as_object().cloned()) {
            for (k, v) in packed {
                merged.entry(k).or_insert(v);
            }
        }
    }
    merged
}

/// Parse a `tasks/send` request — JSON-RPC envelope or flat shape.
pub fn parse_send_request(body: &Value) -> Result<SendRequest, SendError> {
    let jsonrpc = body.get("jsonrpc").is_some()
        || body.get("params").and_then(|p| p.get("message")).is_some();
    let message = request_message(body);
    let metadata = request_metadata(body, message);

    let skill = metadata
        .get("skill")
        .and_then(|v| v.as_str())
        .or_else(|| body.get("skill").and_then(|v| v.as_str()))
        .map(String::from)
        .ok_or_else(|| SendError {
            code: 4001,
            message: "skill not in request (looked in metadata.skill and skill)".to_string(),
        })?;

    let task_id = body
        .get("params")
        .and_then(|p| p.get("id"))
        .and_then(|v| v.as_str())
        .or_else(|| body.get("id").and_then(|v| v.as_str()))
        .map(String::from)
        .unwrap_or_else(new_task_id);

    // The context is the message object; the resolved metadata is merged
    // back into it so a skill can read `metadata.skill` without decoding
    // the pack itself.
    let mut ctx = message.cloned().unwrap_or_else(|| json!({}));
    if !ctx.is_object() {
        ctx = json!({});
    }
    if let Some(obj) = ctx.as_object_mut() {
        obj.insert("metadata".to_string(), Value::Object(metadata.clone()));
        if !obj.contains_key("user_id") {
            if let Some(user_id) = metadata.get("user_id").and_then(|v| v.as_str()) {
                obj.insert("user_id".to_string(), Value::String(user_id.to_string()));
            }
        }
    }
    // `JsonCtx` requires `user_id`; a request without an identity is
    // rejected here instead of failing later with an opaque parse error.
    if ctx.get("user_id").and_then(|v| v.as_str()).is_none() {
        return Err(SendError {
            code: 4002,
            message:
                "user_id missing from the request (expected in the context or in metadata.user_id)"
                    .to_string(),
        });
    }

    Ok(SendRequest {
        task_id,
        skill,
        ctx,
        jsonrpc,
        rpc_id: body.get("id").cloned().unwrap_or(Value::Null),
    })
}

/// The payload of a skill, as the text an A2A client reads.
fn payload_text(payload: &Value) -> String {
    match payload {
        Value::String(s) => s.clone(),
        other => serde_json::to_string(other).unwrap_or_else(|_| other.to_string()),
    }
}

/// A fresh task id (the request did not carry one).
fn new_task_id() -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    let ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    format!("task-{ms}-{n}")
}

/// Dispatch one `tasks/send` request and shape the answer for the caller.
async fn handle_send(skills: Arc<StatelessSkillRouter>, body: Value) -> Value {
    let request = match parse_send_request(&body) {
        Ok(r) => r,
        Err(e) => return json!({"error": {"code": e.code, "message": e.message}}),
    };
    match skills.dispatch(&request.skill, request.ctx.clone()).await {
        Ok(out) => {
            if request.jsonrpc {
                json!({
                    "jsonrpc": "2.0",
                    "id": request.rpc_id,
                    "result": {
                        "task": {
                            "id": request.task_id,
                            "status": {
                                "state": "completed",
                                "message": payload_text(&out.payload),
                                "timestamp": chrono::Utc::now().timestamp_millis(),
                            }
                        }
                    }
                })
            } else {
                // The flat shape keeps the raw payload — old callers read
                // it that way.
                out.payload
            }
        }
        Err(e) => json!({"error": {"code": 500, "message": e}}),
    }
}

pub fn build_a2a_router(
    card: AgentCard,
    skills: Arc<StatelessSkillRouter>,
) -> Router {
    let card = Arc::new(card);
    let send = post({
        let skills = Arc::clone(&skills);
        move |Json(body): Json<Value>| {
            let skills = Arc::clone(&skills);
            async move { Json(handle_send(skills, body).await) }
        }
    });
    Router::new()
        .route("/.well-known/agent.json", get({
            let card = Arc::clone(&card);
            move || {
                let card = Arc::clone(&card);
                async move { Json(serde_json::to_value(&*card).unwrap_or_else(|_| json!({}))) }
            }
        }))
        .route("/a2a/tasks/send", send.clone())
        .route("/api/a2a", send.clone())
        .route("/", send)
        .route("/a2a/tasks/{id}", get(|| async { Json(json!({"status": "not_implemented"})) }))
        .route("/a2a/tasks/{id}/cancel", post(|| async { Json(json!({"ok": true})) }))
}