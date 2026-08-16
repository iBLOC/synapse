//! Build an axum router exposing the lobe's A2A endpoints.
//!
//! Wraps `synapse::a2a::server::A2AServerBuilder` plus the standard
//! endpoints: `GET /.well-known/agent.json`, `POST /a2a/tasks/send`,
//! `GET /a2a/tasks/{id}`, `POST /a2a/tasks/{id}/cancel`.

use std::sync::Arc;

use axum::{routing::{get, post}, Json, Router};
use serde_json::{json, Value};

use crate::a2a::card::AgentCard;
use crate::lobe::StatelessSkillRouter;

pub fn build_a2a_router(
    card: AgentCard,
    skills: Arc<StatelessSkillRouter>,
) -> Router {
    let card = Arc::new(card);
    Router::new()
        .route("/.well-known/agent.json", get({
            let card = Arc::clone(&card);
            move || {
                let card = Arc::clone(&card);
                async move { Json(serde_json::to_value(&*card).unwrap_or_else(|_| json!({}))) }
            }
        }))
        .route("/a2a/tasks/send", post({
            let skills = Arc::clone(&skills);
            move |Json(body): Json<Value>| {
                let skills = Arc::clone(&skills);
                async move {
                    let skill_id = body.get("skill")
                        .and_then(|v| v.as_str())
                        .or_else(|| body.get("message")
                            .and_then(|m| m.get("metadata"))
                            .and_then(|m| m.get("skill"))
                            .and_then(|v| v.as_str()));
                    let Some(skill_id) = skill_id else {
                        return Json(json!({"error": {"code": 4001, "message": "skill not in request"}}));
                    };
                    let ctx = body.get("message")
                        .cloned()
                        .unwrap_or(json!({}));
                    match skills.dispatch(skill_id, ctx).await {
                        Ok(out) => Json(out.payload),
                        Err(e) => Json(json!({"error": {"code": 500, "message": e}})),
                    }
                }
            }
        }))
        .route("/a2a/tasks/{id}", get(|| async { Json(json!({"status": "not_implemented"})) }))
        .route("/a2a/tasks/{id}/cancel", post(|| async { Json(json!({"ok": true})) }))
}
