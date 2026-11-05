//! HTTP-over-ZitiStream — serve an axum `Router` as a dark service.
//!
//! The piece that was the open `// TODO` in
//! `tiangong-api::ziti::dark_service::serve` and
//! `tiangong-agent::ziti::dark_service::serve` before this module
//! landed.
//!
//! ## Public API
//!
//! Most callers want [`serve`], which does identity-load +
//! service-listen + accept-loop in a single call:
//!
//! ```no_run
//! use synapse::transport::ziti;
//! # async fn run(router: axum::Router) -> anyhow::Result<()> {
//! ziti::serve(router, "/etc/encephalon/identity.json", "encephalon").await?;
//! # Ok(()) }
//! ```
//!
//! [`serve_ziti_stream`] is the per-connection helper for callers
//! that already manage their own `ZitiStream` (e.g. inside a custom
//! `DarkServiceHandler`).

use std::convert::Infallible;
use std::pin::Pin;
use std::sync::Arc;

use hyper::server::conn::http1::Builder;
use hyper::service::Service;
use hyper::{Request, Response};
use hyper_util::rt::TokioIo;
use tower::util::ServiceExt;
use tracing::{debug, error, info};
use super::gateway::ZitiGateway;
use super::ZitiError;

/// Axum's `Router` wrapped as a `hyper::service::Service`.
#[derive(Clone)]
struct AxumService {
    inner: axum::Router,
}

impl Service<Request<hyper::body::Incoming>> for AxumService {
    type Response = Response<axum::body::Body>;
    type Error = Infallible;
    type Future = Pin<
        Box<dyn std::future::Future<Output = Result<Self::Response, Self::Error>> + Send>,
    >;
    fn call(&self, req: Request<hyper::body::Incoming>) -> Self::Future {
        let inner = self.inner.clone();
        Box::pin(async move {
            let resp = inner
                .oneshot(req)
                .await
                .expect("axum::Router oneshot is infallible");
            Ok::<_, Infallible>(resp)
        })
    }
}

/// Serve one axum `Router` over one accepted stream until the peer
/// disconnects.
///
/// ## ⚠️ IO 是**泛型**的 —— 这不是洁癖，是**为了让这件事可验证**
///
/// `S` 只需要是「一条类 TCP 的双向流」⇒ ⇒ `ZitiStream` 满足 ⇒ ⇒
/// **普通 `tokio::net::TcpStream` 也满足** ⇒ ⇒ 于是**同一条代码路径**
/// 可以在 loopback TCP 上**真跑**（见本文件的 `mod tests`）⇒ ⇒
/// 「这条承载形态支持 WebSocket / SSE」这句话**有运行时证据**，
/// 而不只是「编译过」。
///
/// ⇒ ⇒ 以前这里写死 `ZitiStream` ⇒ ⇒ 那条路径在**没有 ziti 网络**的机器上
/// **没法验证** ⇒ ⇒ 只能停在编译级 —— 那种「编译过」经常是假的。
///
/// ## ⚠️ `.with_upgrades()` 是「WebSocket 能不能过」的那一行
///
/// 没有它，hyper 收到 `101 Switching Protocols` 之后**不把连接交还**
/// ⇒ ⇒ 升级握手像是完成了，但连接随即被关 ⇒ ⇒ 症状是「握手成功、然后
/// 断」—— 典型的「失败长得像成功」。
///
/// ⇒ ⇒ **对照组**：axum 自己的 `axum::serve` 走
/// `hyper_util::server::conn::auto::Builder`，它的 h1 分支就是
/// `builder.http1.serve_connection(io, service).with_upgrades()`
/// （hyper-util 0.1.20 `server/conn/auto/mod.rs:679`）⇒ ⇒ **同一张
/// Router 挂在 `axum::serve` 上能跑 WebSocket，挂在这里之前不能 ——
/// 差别只有这一行**。
pub async fn serve_ziti_stream<S>(stream: S, app: axum::Router)
where
    S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin + Send + 'static,
{
    let service = AxumService { inner: app };
    let io = TokioIo::new(stream);
    let conn = Builder::new()
        .preserve_header_case(true)
        .serve_connection(io, service)
        .with_upgrades();
    if let Err(e) = conn.await {
        debug!(error = %e, "ziti HTTP connection ended");
    }
}

/// Top-level convenience: load identity, register as dark service,
/// accept connections forever, and serve `router` on each.
///
/// Replaces the four-line dance that used to live in every caller:
///
/// ```ignore
/// let gateway = ZitiGateway::new(&config.identity_path).await?;
/// let mut listener = gateway.listen(&config.service_name).await?;
/// loop {
///     let stream = listener.accept().await?;
///     tokio::spawn(serve_ziti_stream(stream, router.clone()));
/// }
/// ```
///
/// Returns `Err(ZitiError::*)` if identity load fails or the
/// initial listen fails. After a successful return from this
/// function's prelude, the loop runs forever — cancellation must
/// come from the caller (e.g. dropping the surrounding
/// `tokio::task::JoinHandle`).
pub async fn serve(
    router: axum::Router,
    identity_path: &str,
    service_name: &str,
) -> Result<(), ZitiError> {
    serve_with_ready(router, identity_path, service_name, || {}).await
}

/// Same as [`serve`], but **it says when it is ready**.
///
/// ⚠️ **为什么需要「就绪」** —— `serve` 建好监听器之后**一直跑到永远**
/// ⇒ ⇒ 调用方拿不到任何「已经 bind 好了」的信号
/// ⇒ ⇒ 于是「宿主刚起来、客户端马上 dial」会红在
/// 「路由器拒绝连接: service … has no terminators」
/// —— **真机上实测过**（账本 ㎞ 条）
/// ⇒ ⇒ 那**不是代码缺陷**，是**缺一个就绪信号**。
///
/// ⇒ ⇒ `on_ready` 在**监听器建好之后**被调用一次
/// （此刻 terminator 已经在路由器上了）
/// ⇒ ⇒ 调用方可以「等就绪再 dial」。
pub async fn serve_with_ready(
    router: axum::Router,
    identity_path: &str,
    service_name: &str,
    on_ready: impl FnOnce() + Send + 'static,
) -> Result<(), ZitiError> {
    let gateway = Arc::new(
        ZitiGateway::new(identity_path)
            .await
            .map_err(|e| ZitiError::IdentityLoadError(e.to_string()))?,
    );
    let mut listener = gateway
        .listen(service_name)
        .await
        .map_err(|e| ZitiError::ListenError(e.to_string()))?;

    info!(service = %service_name, "ziti dark service serving");
    // ⚠️ **就绪点** —— terminator 已经在路由器上了，客户端现在 dial 就能通。
    on_ready();
    loop {
        match listener.accept().await {
            Ok(stream) => {
                let router = router.clone();
                tokio::spawn(async move {
                    serve_ziti_stream(stream, router).await;
                });
            }
            Err(e) => {
                error!(error = %e, "ziti listener accept failed");
                // brief backoff to avoid hot-loop on persistent errors
                tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    //! **承载形态的运行时实证** —— 生产**同一条**代码路径，换成普通 TCP 跑。
    //!
    //! ⚠️ **为什么要绕这一圈** —— ziti 那条路需要一张**真实的零信任网络**
    //! （今天没有）⇒ ⇒ 端到端验不了 ⇒ ⇒ 只能停在「编译过」⇒ ⇒ 而「编译过」
    //! 经常是假的。
    //!
    //! ⇒ ⇒ 但 `ZitiStream` 的语义就是「一条类 TCP 的双向流」⇒ ⇒ 于是把
    //! **同一条代码路径**架在 `tokio::net::TcpStream` 上跑 ⇒ ⇒ 就能真真
    //! 正正地验「这条承载形态支持 WebSocket / SSE」⇒ ⇒ 证据强度从
    //! 「编译过」升到「**跑通过**」。
    //!
    //! ⚠️ **它证明什么、不证明什么** ——
    //!
    //! - ✅ **证明**：HTTP over 一条类 TCP 流 —— WebSocket 升级、SSE 分块
    //!   到达、普通请求/响应 —— **都真的能跑**。
    //! - ❌ **不证明**：ziti 网络本身能通（那需要真实的 controller ＋
    //!   edge router，本轮环境没有）。

    use super::*;
    use axum::extract::ws::{WebSocket, WebSocketUpgrade};
    use axum::response::sse::{Event, Sse};
    use axum::routing::get;
    use futures::SinkExt;
    use futures::StreamExt;
    use http_body_util::BodyExt;
    use hyper_util::rt::TokioIo;
    use std::convert::Infallible;
    use std::time::{Duration, Instant};
    use tokio::net::{TcpListener, TcpStream};

    /// 起一个 loopback TCP 上的 HTTP 服务 —— 跑的是**生产同一条**路径。
    async fn spawn_server(app: axum::Router) -> u16 {
        let listener = TcpListener::bind(("127.0.0.1", 0)).await.expect("bind");
        let port = listener.local_addr().expect("addr").port();
        tokio::spawn(async move {
            while let Ok((stream, _)) = listener.accept().await {
                let app = app.clone();
                tokio::spawn(async move {
                    serve_ziti_stream(stream, app).await;
                });
            }
        });
        port
    }

    /// ⚠️ **对照组** —— 与生产路径**只差** `.with_upgrades()`。
    ///
    /// 只在测试里存在：它证明生产路径的 WebSocket 能力**真的**是那一行
    /// 给的，而不是「本来就有」。
    async fn serve_without_upgrades<S>(stream: S, app: axum::Router)
    where
        S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin + Send + 'static,
    {
        let service = AxumService { inner: app };
        let io = TokioIo::new(stream);
        if let Err(e) = Builder::new()
            .preserve_header_case(true)
            .serve_connection(io, service)
            .await
        {
            debug!(error = %e, "comparison connection ended");
        }
    }

    async fn spawn_server_without_upgrades(app: axum::Router) -> u16 {
        let listener = TcpListener::bind(("127.0.0.1", 0)).await.expect("bind");
        let port = listener.local_addr().expect("addr").port();
        tokio::spawn(async move {
            while let Ok((stream, _)) = listener.accept().await {
                let app = app.clone();
                tokio::spawn(async move {
                    serve_without_upgrades(stream, app).await;
                });
            }
        });
        port
    }

    fn app() -> axum::Router {
        axum::Router::new()
            .route("/plain", get(|| async { "plain-ok" }))
            .route("/ws", get(ws_handler))
            .route("/sse", get(sse_handler))
    }

    async fn ws_handler(ws: WebSocketUpgrade) -> impl axum::response::IntoResponse {
        ws.on_upgrade(|mut socket: WebSocket| async move {
            while let Some(Ok(msg)) = socket.next().await {
                if socket.send(msg).await.is_err() {
                    break;
                }
            }
        })
    }

    /// SSE：第一个 tick **立刻**发，第二个 **120ms 后**发 ⇒ ⇒
    /// ⇒ 「两块之间隔 ≥80ms」就证明**没有整体缓冲**。
    async fn sse_handler() -> Sse<impl futures::Stream<Item = Result<Event, Infallible>>> {
        let stream = futures::stream::unfold(0u32, |i| async move {
            if i >= 2 {
                return None;
            }
            if i == 1 {
                tokio::time::sleep(Duration::from_millis(120)).await;
            }
            Some((Ok(Event::default().data(format!("tick-{i}"))), i + 1))
        });
        Sse::new(stream)
    }

    #[derive(Debug)]
    enum WsOutcome {
        HandshakeFailed,
        ExchangeFailed,
        ExchangeSucceeded,
    }

    /// 跑一次 WebSocket 握手 ＋ 一来一回，返回**它到底发生了什么**。
    async fn ws_roundtrip(port: u16) -> WsOutcome {
        let url = format!("ws://127.0.0.1:{port}/ws");
        let connected = tokio::time::timeout(
            Duration::from_secs(10),
            tokio_tungstenite::connect_async(url.as_str()),
        )
        .await;
        match connected {
            Err(_) => WsOutcome::ExchangeFailed, // 超时 ⇒ 挂住 ⇒ 一样算不通
            Ok(Err(_)) => WsOutcome::HandshakeFailed,
            Ok(Ok((mut socket, _))) => {
                if socket
                    .send(tokio_tungstenite::tungstenite::Message::Text(
                        "ping".into(),
                    ))
                    .await
                    .is_err()
                {
                    return WsOutcome::ExchangeFailed;
                }
                match tokio::time::timeout(Duration::from_secs(10), socket.next()).await {
                    Err(_) => WsOutcome::ExchangeFailed,
                    Ok(None) => WsOutcome::ExchangeFailed,
                    Ok(Some(Err(_))) => WsOutcome::ExchangeFailed,
                    Ok(Some(Ok(_))) => WsOutcome::ExchangeSucceeded,
                }
            }
        }
    }

    /// 取下一个**非空数据帧** ⇒ ⇒ 跳过空帧与 trailers 帧。
    ///
    /// ⚠️ **不用 `BodyExt::data()`** —— synapse 锁定的 `http-body-util`
    /// **0.1.3** 的 `BodyExt` 只有 `frame()` / `collect()`（`data()` 是
    /// 0.1.5 才加的）⇒ ⇒ 用 `frame()` 才编得过。
    async fn next_data(body: &mut hyper::body::Incoming) -> bytes::Bytes {
        loop {
            let frame = body
                .frame()
                .await
                .expect("帧读取成功")
                .expect("帧读取失败");
            if let Ok(data) = frame.into_data() {
                if !data.is_empty() {
                    return data;
                }
            }
        }
    }
    /// ✅ **WebSocket 升级真跑**：握手 ＋ 一来一回。
    #[tokio::test]
    async fn websocket_upgrade_works_over_the_production_path() {
        let port = spawn_server(app()).await;
        match ws_roundtrip(port).await {
            WsOutcome::ExchangeSucceeded => {}
            other => panic!("生产路径的 WebSocket 应当能收发，实际：{other:?}"),
        }
    }

    /// ⚠️ **对照组** —— 只差 `.with_upgrades()` ⇒ ⇒ WebSocket **过不去**。
    ///
    /// ⚠️ **症状是「握手像是成功了、然后断」** ⇒ ⇒ 断言**不能**只写
    /// 「握手失败」（那会假红）⇒ ⇒ 这里三种结局都判「不通」，只有
    /// 「收发成功」才判失败。
    #[tokio::test]
    async fn without_upgrades_websocket_cannot_work() {
        let port = spawn_server_without_upgrades(app()).await;
        match ws_roundtrip(port).await {
            WsOutcome::ExchangeSucceeded => {
                panic!("对照组居然收发成功了 ⇒ ⇒ `.with_upgrades()` 不是那根开关")
            }
            _ => {}
        }
    }

    /// ✅ **SSE 是分块到达的** —— 不是「等整个响应收完再给」。
    ///
    /// ⚠️ 判据是「**两块之间隔 ≥80ms**」而不是「总耗时」⇒ ⇒ 因为整体缓冲
    /// 的情况下两块会**同时**到达（间隔 ≈ 0）⇒ ⇒ 只有间隔才能区分两者。
    #[tokio::test]
    async fn sse_body_arrives_in_chunks_not_buffered() {
        let port = spawn_server(app()).await;
        let stream = TcpStream::connect(("127.0.0.1", port)).await.expect("connect");
        let io = TokioIo::new(stream);
        let (mut sender, conn) = hyper::client::conn::http1::handshake(io)
            .await
            .expect("client handshake");
        tokio::spawn(async move {
            let _ = conn.await;
        });

        let req = hyper::Request::builder()
            .uri("/sse")
            .header("Host", "127.0.0.1")
            .body(axum::body::Body::empty())
            .expect("req");
        let resp = sender.send_request(req).await.expect("SSE 响应");
        assert_eq!(resp.status(), 200);

        let mut body = resp.into_body();
        let t0 = Instant::now();
        let first = tokio::time::timeout(Duration::from_secs(5), next_data(&mut body))
            .await
            .expect("第一块应当到达");
        let t_first = t0.elapsed();
        let second = tokio::time::timeout(Duration::from_secs(5), next_data(&mut body))
            .await
            .expect("第二块应当到达");
        let gap = t0.elapsed() - t_first;

        let text = format!(
            "{}{}",
            String::from_utf8_lossy(&first),
            String::from_utf8_lossy(&second)
        );
        assert!(
            text.contains("tick-0") && text.contains("tick-1"),
            "两块内容不对：{text}"
        );
        assert!(
            gap >= Duration::from_millis(80),
            "SSE 两块之间应隔 ≥80ms（真流式）；实测 {gap:?} ⇒ ⇒ 被整体缓冲了"
        );
    }

    /// ✅ **普通请求/响应没被 upgrades 影响**（回归）。
    #[tokio::test]
    async fn plain_request_still_works() {
        let port = spawn_server(app()).await;
        let stream = TcpStream::connect(("127.0.0.1", port)).await.expect("connect");
        let io = TokioIo::new(stream);
        let (mut sender, conn) = hyper::client::conn::http1::handshake(io)
            .await
            .expect("handshake");
        tokio::spawn(async move {
            let _ = conn.await;
        });
        let req = hyper::Request::builder()
            .uri("/plain")
            .header("Host", "127.0.0.1")
            .body(axum::body::Body::empty())
            .expect("req");
        let resp = sender.send_request(req).await.expect("响应");
        assert_eq!(resp.status(), 200);
        let body = resp.into_body().collect().await.expect("body").to_bytes();
        assert_eq!(String::from_utf8_lossy(&body), "plain-ok");
    }
}
