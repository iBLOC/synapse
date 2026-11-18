//! 回环端点不许走系统代理 —— 而这条判据**不依赖本机有没有代理**。
//!
//! ❶ **自造一个假代理**：在回环上起一个小服务，**永远回 502 空 body**
//!    （这正是真代理够不到本机回环时的签名）⇒ ⇒ 再把 `HTTP_PROXY` 指向它。
//! ❷ **自造一个假上游**：在回环上起一个回 `200` 的服务。
//! ❸ 于是：**没有这条修法**的客户端 ⇒ ⇒ 被假代理截（**502**）；
//!    **有这条修法**的（`http_client::client_for`）⇒ ⇒ 直接打到假上游（**200**）。
//!
//! ⇒ ⇒ 结论**与本机注册表里的系统代理无关**：任何机器、任何 CI，同一个结论
//! ⇒ ⇒ 不会像「靠本机恰好开着代理才红」那样变成假绿。
//!
//! ⚠⚠ **`NO_PROXY` 必须写成一条不含回环的值**（`blocked.invalid`）：
//! hyper-util 先读环境变量，**只有在环境变量为空时**才去读注册表的
//! `ProxyOverride` ⇒ ⇒ 环境变量一旦非空，注册表那份就**整份作废**。
//! 写成 `127.0.0.1` 的话，假上游自己就被绕过了 ⇒ ⇒ 负控会假绿。

use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use synapse::providers::LLMProvider;

/// 改环境的判据共用一把锁（同一个进程里判据是并发跑的）。
///
/// ⚠ 用 tokio 的锁而不是 std 的：std 的 `MutexGuard` 跨 `.await` 会被
/// clippy 判红（`await_holding_lock`）⇒ ⇒ 而这里**必须**拿着锁发请求
/// （环境变量是在发请求那一刻被读的）。
static ENV_LOCK: std::sync::LazyLock<tokio::sync::Mutex<()>> =
    std::sync::LazyLock::new(|| tokio::sync::Mutex::new(()));

/// 起一个回环小服务：每来一个连接就回一份固定响应，并数命中次数。
fn spawn_responder(response: String) -> (String, Arc<AtomicUsize>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    let hits = Arc::new(AtomicUsize::new(0));
    let seen = Arc::clone(&hits);
    std::thread::spawn(move || {
        for stream in listener.incoming() {
            let mut sock = match stream {
                Ok(s) => s,
                Err(_) => break,
            };
            seen.fetch_add(1, Ordering::SeqCst);
            let mut buf = [0u8; 8192];
            let _ = sock.read(&mut buf);
            let _ = sock.write_all(response.as_bytes());
            let _ = sock.flush();
        }
    });
    (format!("http://{addr}"), hits)
}

fn ok_response(body: &str) -> String {
    format!(
        "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        body.len(),
        body
    )
}

/// 真代理够不到本机回环时的签名：**502 + 空 body**。
fn proxy_response() -> String {
    "HTTP/1.1 502 Bad Gateway\r\nContent-Length: 0\r\nConnection: close\r\n\r\n".to_string()
}

fn set_env(key: &str, value: &str) {
    // SAFETY: 本文件里改环境的判据都持 `ENV_LOCK`，同一时刻只有一个在改。
    unsafe { std::env::set_var(key, value) }
}

fn remove_env(key: &str) {
    // SAFETY: 同上。
    unsafe { std::env::remove_var(key) }
}

/// 把假代理挂上（并把注册表那份 `ProxyOverride` 整份作废）。
fn arm_fake_proxy(proxy_url: &str) {
    set_env("HTTP_PROXY", proxy_url);
    set_env("HTTPS_PROXY", proxy_url);
    set_env("NO_PROXY", "blocked.invalid");
}

fn disarm_fake_proxy() {
    remove_env("HTTP_PROXY");
    remove_env("HTTPS_PROXY");
    remove_env("NO_PROXY");
}

#[tokio::test]
async fn a_loopback_peer_is_not_handed_to_the_proxy() {
    let _guard = ENV_LOCK.lock().await;
    let (upstream, up_hits) = spawn_responder(ok_response("UPSTREAM"));
    let (proxy, proxy_hits) = spawn_responder(proxy_response());

    arm_fake_proxy(&proxy);
    let client = synapse::http_client::client_for(&upstream);
    disarm_fake_proxy();

    let resp = client
        .get(&upstream)
        .send()
        .await
        .expect("the loopback peer must answer");
    assert_eq!(resp.status().as_u16(), 200, "the loopback peer must not be proxied");
    let text = resp.text().await.unwrap_or_default();
    assert!(text.contains("UPSTREAM"), "actual: {text}");
    assert_eq!(proxy_hits.load(Ordering::SeqCst), 0, "the proxy must not have been used");
    assert_eq!(up_hits.load(Ordering::SeqCst), 1, "the upstream must have answered directly");
}

#[tokio::test]
async fn without_the_bypass_the_proxy_would_have_swallowed_the_loopback_peer() {
    let _guard = ENV_LOCK.lock().await;
    let (upstream, up_hits) = spawn_responder(ok_response("UPSTREAM"));
    let (proxy, proxy_hits) = spawn_responder(proxy_response());

    arm_fake_proxy(&proxy);
    // ⚠ 这是**没有**这条修法的客户端（reqwest 自己的默认行为）。
    let client = reqwest::Client::new();
    disarm_fake_proxy();

    let resp = client
        .get(&upstream)
        .send()
        .await
        .expect("the fake proxy answers");
    assert_eq!(
        resp.status().as_u16(),
        502,
        "without the bypass the proxy swallows the loopback peer"
    );
    assert_eq!(proxy_hits.load(Ordering::SeqCst), 1, "the proxy must have answered");
    assert_eq!(up_hits.load(Ordering::SeqCst), 0, "the upstream must not have been reached");
}

#[tokio::test]
async fn a_remote_peer_still_goes_through_the_proxy() {
    let _guard = ENV_LOCK.lock().await;
    let (_upstream, _up_hits) = spawn_responder(ok_response("UPSTREAM"));
    let (proxy, proxy_hits) = spawn_responder(proxy_response());

    arm_fake_proxy(&proxy);
    // ⚠ 远端对端**不许**一起被绕开（有些部署真要把远端走代理）。
    let client = synapse::http_client::client_for("http://peer.invalid/v1/models");
    disarm_fake_proxy();

    let resp = client
        .get("http://peer.invalid/v1/models")
        .send()
        .await
        .expect("the fake proxy answers");
    assert_eq!(resp.status().as_u16(), 502, "the remote peer went through the proxy");
    assert!(
        proxy_hits.load(Ordering::SeqCst) >= 1,
        "a remote peer must keep the system proxy"
    );
}

#[tokio::test]
async fn the_lobe_llm_client_reaches_a_loopback_peer() {
    let _guard = ENV_LOCK.lock().await;
    let body = r#"{"choices":[{"message":{"content":"LLM_ANSWER"}}]}"#;
    let (upstream, up_hits) = spawn_responder(ok_response(body));
    let (proxy, proxy_hits) = spawn_responder(proxy_response());

    arm_fake_proxy(&proxy);
    let llm = synapse::lobe::provider_set::OpenAILlm::new(upstream.clone(), "test-model".into());
    disarm_fake_proxy();

    let answer = llm
        .chat(
            vec![synapse::providers::Message::user("hello")],
            synapse::providers::ChatOptions::default(),
        )
        .await
        .expect("the loopback peer must answer");
    assert_eq!(answer.text(), "LLM_ANSWER");
    assert_eq!(proxy_hits.load(Ordering::SeqCst), 0, "the proxy must not have been used");
    assert_eq!(up_hits.load(Ordering::SeqCst), 1, "the upstream must have answered directly");
}

#[tokio::test]
async fn the_registry_registration_reaches_a_loopback_peer() {
    let _guard = ENV_LOCK.lock().await;
    let (upstream, up_hits) = spawn_responder(ok_response("{}"));
    let (proxy, proxy_hits) = spawn_responder(proxy_response());

    arm_fake_proxy(&proxy);
    let card = synapse::lobe::AgentCard::new("test", "test agent", &upstream, "1.0.0");
    let outcome = synapse::lobe::service_registration::register_with_registry(&upstream, &card).await;
    disarm_fake_proxy();

    assert!(outcome.is_ok(), "the registration must reach the loopback registry: {outcome:?}");
    assert_eq!(proxy_hits.load(Ordering::SeqCst), 0, "the proxy must not have been used");
    assert_eq!(up_hits.load(Ordering::SeqCst), 1, "the upstream must have answered directly");
}
