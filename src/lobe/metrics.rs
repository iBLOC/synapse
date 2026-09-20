//! 通用 Prometheus metrics 模块 — 替代各 lobe 重复的 metrics_exporter.rs。
//!
//! ## 用法
//!
//! ```ignore
//! use synapse::lobe::metrics;
//!
//! // 1. 启动时 install（幂等）
//! metrics::install();
//!
//! // 2. 启动 metrics HTTP endpoint
//! metrics::spawn_server("0.0.0.0:9301".parse().unwrap());
//!
//! // 3. 记录指标
//! metrics::counter("request_count", &[("skill", "face_recognition")]).increment(1);
//! metrics::histogram("latency_ms", &[("op", "detect")]).record(42.0);
//! ```
//!
//! 所有 lobe 应使用本模块而非自建 metrics_exporter。

use std::net::SocketAddr;
use std::sync::OnceLock;

#[cfg(feature = "metrics")]
use axum::{routing::get, Router};
#[cfg(feature = "metrics")]
use metrics::{counter, describe_counter, describe_histogram, histogram};
#[cfg(feature = "metrics")]
use metrics_exporter_prometheus::{PrometheusBuilder, PrometheusHandle};

#[cfg(feature = "metrics")]
static HANDLE: OnceLock<PrometheusHandle> = OnceLock::new();

#[cfg(feature = "metrics")]
pub fn install() {
    if HANDLE.get().is_some() {
        return;
    }
    let handle = PrometheusBuilder::new()
        .install_recorder()
        .expect("install prometheus recorder");
    let _ = HANDLE.set(handle);
}

#[cfg(feature = "metrics")]
pub fn router() -> Router {
    let handle = HANDLE.get().cloned().expect("metrics::install not called");
    Router::new().route(
        "/metrics",
        get(move || {
            let h = handle.clone();
            async move { h.render() }
        }),
    )
}

#[cfg(feature = "metrics")]
pub async fn serve(addr: SocketAddr) -> std::io::Result<()> {
    install();
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, router()).await
}

#[cfg(feature = "metrics")]
pub fn spawn_server(addr: SocketAddr) {
    tokio::spawn(async move {
        if let Err(e) = serve(addr).await {
            tracing::error!("metrics server error: {e}");
        }
    });
}

/// 描述一个 counter metric。
#[cfg(feature = "metrics")]
pub fn describe_counter(name: &'static str, help: &'static str) {
    describe_counter!(name, help);
}

/// 描述一个 histogram metric。
#[cfg(feature = "metrics")]
pub fn describe_histogram(name: &'static str, help: &'static str) {
    describe_histogram!(name, help);
}

/// 增加带 skill label 的 counter。
#[cfg(feature = "metrics")]
pub fn increment_skill(name: &'static str, skill: &'static str) {
    counter!(name, "skill" => skill).increment(1);
}

/// 增加带 op label 的 counter。
#[cfg(feature = "metrics")]
pub fn increment_op(name: &'static str, op: &'static str) {
    counter!(name, "op" => op).increment(1);
}

/// 记录带 op label 的 histogram。
#[cfg(feature = "metrics")]
pub fn record_op(name: &'static str, value: f64, op: &'static str) {
    histogram!(name, "op" => op).record(value);
}

/// 记录带 skill label 的 histogram。
#[cfg(feature = "metrics")]
pub fn record_skill(name: &'static str, value: f64, skill: &'static str) {
    histogram!(name, "skill" => skill).record(value);
}

/// 当 `metrics` feature 未启用时，提供 stub API。
#[cfg(not(feature = "metrics"))]
pub fn install() {}

#[cfg(not(feature = "metrics"))]
pub async fn serve(_addr: SocketAddr) -> std::io::Result<()> {
    Ok(())
}

#[cfg(not(feature = "metrics"))]
pub fn spawn_server(_addr: SocketAddr) {}

#[cfg(not(feature = "metrics"))]
pub fn describe_counter(_name: &'static str, _help: &'static str) {}

#[cfg(not(feature = "metrics"))]
pub fn describe_histogram(_name: &'static str, _help: &'static str) {}

#[cfg(not(feature = "metrics"))]
pub fn increment_skill(_name: &'static str, _skill: &'static str) {}

#[cfg(not(feature = "metrics"))]
pub fn increment_op(_name: &'static str, _op: &'static str) {}

#[cfg(not(feature = "metrics"))]
pub fn record_op(_name: &'static str, _value: f64, _op: &'static str) {}

#[cfg(not(feature = "metrics"))]
pub fn record_skill(_name: &'static str, _value: f64, _skill: &'static str) {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stub_api_compiles_without_metrics_feature() {
        install();
        describe_counter("test", "test");
        describe_histogram("test", "test");
        increment_skill("test", "x");
        increment_op("test", "y");
        record_op("test", 1.0, "y");
        record_skill("test", 1.0, "x");
    }
}