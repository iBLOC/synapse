//! Zero-trust **local proxy** —— let the existing frontends reach a remote
//! brain over ziti **without changing a single line of frontend code**.
//!
//! ## The problem it solves
//!
//! The desktop frontend talks HTTP with `fetch(baseUrl + path)`; the mobile
//! frontend talks HTTP with `package:http`. Both hard-code a **loopback**
//! address ⇒ ⇒ neither of them can reach a brain that lives on another
//! machine inside the zero-trust network.
//!
//! ⇒ ⇒ Two ways out:
//! ① rewrite both frontends to speak "ziti" —— impossible in Dart/TS, neither
//! has a ziti SDK;
//! ② **keep the frontends exactly as they are** and put a small **local
//! proxy** in front: it accepts ordinary HTTP/WebSocket on loopback and
//! re-sends everything over a ziti stream.
//!
//! ⇒ ⇒ ② is this file. The frontend's `baseUrl` does not change — only
//! **what is listening on that port** changes (brain face ⇒ zero-trust
//! proxy).
//!
//! ## ⚠️ Three protocols must survive the trip
//!
//! | protocol | how it survives |
//! |---|---|
//! | request/response | forwarded as-is |
//! | **SSE** | body is relayed **frame by frame** ⇒ ⇒ never buffered |
//! | **WebSocket** | local upgrade ＋ upstream upgrade, frames relayed both ways |
//!
//! ⚠️ A proxy that buffers the body silently **breaks SSE** ⇒ ⇒ the test
//! below asserts the **gap between two chunks** (buffering ⇒ both arrive
//! together).
//!
//! ## ⚠️ The upstream is pluggable —— that's what makes it testable
//!
//! `Connector` is "open one stream to the upstream". The production one
//! dials ziti; the test one dials plain TCP ⇒ ⇒ **the same proxy code** is
//! exercised end to end without a zero-trust network.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;

use axum::body::Body;
use axum::extract::ws::{Message as AxumMessage, WebSocket, WebSocketUpgrade};
use axum::extract::{FromRequestParts, Request, State};
use axum::response::{IntoResponse, Response};
use axum::routing::any;
use futures::future::BoxFuture;
use futures::{SinkExt, StreamExt};
use hyper::http::header::{HeaderName, HeaderValue, UPGRADE};
use hyper::http::{HeaderMap, StatusCode};
use http_body_util::BodyExt;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio_tungstenite::tungstenite as ts;

use super::http_client::request_over_stream_bytes;
use super::ws_client::connect_ws_over_stream;

/// A duplex upstream stream (a ziti stream in production, TCP in tests).
pub trait Upstream: AsyncRead + AsyncWrite + Unpin + Send {}
impl<T: AsyncRead + AsyncWrite + Unpin + Send> Upstream for T {}

/// Open one stream to the upstream.
///
/// ⚠️ Returns a **boxed trait object** ⇒ ⇒ the proxy can hold "some duplex
/// stream" without knowing whether it came from ziti or from TCP ⇒ ⇒ that
/// is what lets the same code be tested without a zero-trust network.
pub type Connector = Arc<
    dyn Fn() -> BoxFuture<'static, Result<Box<dyn Upstream>, anyhow::Error>> + Send + Sync,
>;

/// Everything the proxy needs to forward a request.
#[derive(Clone)]
pub struct ProxyState {
    /// Open a stream to the upstream.
    pub connector: Connector,
    /// The `Host` header used on the upstream request.
    pub upstream_host: String,
    /// Headers added to **every** upstream request (door credentials the
    /// frontend cannot know, e.g. the admin token).
    pub default_headers: Vec<(String, String)>,
    /// Request body ceiling in bytes. The proxy buffers the request body
    /// before sending it upstream.
    pub max_body_bytes: usize,
    /// **按服务名路由** —— 一个代理面对**多台**受信设备。
    ///
    /// 路径形状：`/_zt/<service>/<rest>` ⇒ ⇒ 走 `<service>` 那条上游。
    ///
    /// ⚠️ 这是「**大脑可以拆开部署**」的接缝 —— 主机把某个叶区放到
    /// 「另一台设备」上时，那个叶区的远端地址就是
    /// `http://127.0.0.1:<代理口>/_zt/<service>/a2a/tasks/send`。
    ///
    /// ⚠️ 认不出的服务 ⇒ ⇒ **明确 404** ⇒ ⇒ 绝不静默落到默认上游
    /// （那会把请求送到**另一台设备**上 ⇒ ⇒ 失败长得像成功）。
    pub routes: HashMap<String, Connector>,
}

impl ProxyState {
    pub fn new(connector: Connector, upstream_host: impl Into<String>) -> Self {
        Self {
            connector,
            upstream_host: upstream_host.into(),
            default_headers: Vec::new(),
            max_body_bytes: 64 * 1024 * 1024,
            routes: HashMap::new(),
        }
    }

    pub fn with_default_headers(mut self, headers: Vec<(String, String)>) -> Self {
        self.default_headers = headers;
        self
    }

    pub fn with_max_body_bytes(mut self, limit: usize) -> Self {
        self.max_body_bytes = limit;
        self
    }

    /// Add a **named** upstream: `/_zt/<service>/…` reaches it.
    ///
    /// ⇒ ⇒ 一个代理面向多台受信设备 ⇒ ⇒ 「大脑拆开部署」的那些远端
    /// 叶区，各走各的那条上游。
    pub fn with_route(mut self, service: impl Into<String>, connector: Connector) -> Self {
        self.routes.insert(service.into(), connector);
        self
    }
}

/// Which upstream a request path selects.
enum RoutePick {
    /// No service prefix ⇒ the default upstream, path unchanged.
    Default(String),
    /// `/_zt/<service>/<rest>` ⇒ that service's upstream, path = `/<rest>`.
    Service(Connector, String),
    /// `/_zt/<service>/…` but the service is unknown ⇒ ⇒ **explicit failure**.
    UnknownService(String),
}

/// Split `/_zt/<service>/<rest>` into (upstream, path).
///
/// ⚠️ 未知服务 ⇒ ⇒ `UnknownService` ⇒ ⇒ 调用方**明确 404** ⇒ ⇒ 绝不
/// 静默落到默认上游（那会把请求送到**另一台设备**上）。
fn split_route(path: &str, routes: &HashMap<String, Connector>) -> RoutePick {
    const PREFIX: &str = "/_zt/";
    let Some(rest) = path.strip_prefix(PREFIX) else {
        return RoutePick::Default(path.to_string());
    };
    let (service, tail) = match rest.split_once('/') {
        Some((s, t)) => (s, t),
        None => (rest, ""),
    };
    match routes.get(service) {
        Some(connector) => RoutePick::Service(connector.clone(), format!("/{tail}")),
        None => RoutePick::UnknownService(service.to_string()),
    }
}

/// Build a connector that dials a **ziti** dark service.
///
/// ⚠️ Every request opens a **new** ziti stream ⇒ ⇒ that is the honest shape
/// of a dark service (the service is published, not tunnelled).
pub fn ziti_connector(gateway: Arc<super::gateway::ZitiGateway>, service: String) -> Connector {
    Arc::new(move || {
        let gateway = gateway.clone();
        let service = service.clone();
        Box::pin(async move {
            let stream = gateway
                .dial(&service)
                .await
                .map_err(|e| anyhow::anyhow!("拨 ziti 服务 {service} 失败: {e}"))?;
            Ok(Box::new(stream) as Box<dyn Upstream>)
        })
    })
}

/// Build a connector that dials a **plain TCP** upstream.
///
/// ⚠️ This is the **test double** for [`ziti_connector`] ⇒ ⇒ the very same
/// proxy logic runs over an ordinary TCP socket ⇒ ⇒ the proxy can be proven
/// **without** a zero-trust network ⇒ ⇒ and a host can point the proxy at a
/// non-ziti upstream while debugging.
pub fn tcp_connector(port: u16) -> Connector {
    Arc::new(move || {
        let addr = format!("127.0.0.1:{port}");
        Box::pin(async move {
            let stream = tokio::net::TcpStream::connect(&addr)
                .await
                .map_err(|e| anyhow::anyhow!("connect: {e}"))?;
            Ok(Box::new(stream) as Box<dyn Upstream>)
        })
    })
}

/// The proxy's router —— every path is forwarded.
pub fn proxy_router(state: ProxyState) -> axum::Router {
    axum::Router::new()
        .fallback(any(proxy_handler))
        .with_state(state)
}

/// Bind a loopback listener and serve the proxy on it.
///
/// Returns the address it actually bound (the port may differ from the
/// requested one when port 0 is used).
pub async fn spawn_proxy(state: ProxyState, bind: SocketAddr) -> Result<SocketAddr, anyhow::Error> {
    let listener = tokio::net::TcpListener::bind(bind)
        .await
        .map_err(|e| anyhow::anyhow!("零信任代理监听 {bind} 失败: {e}"))?;
    let addr = listener
        .local_addr()
        .map_err(|e| anyhow::anyhow!("读本地地址失败: {e}"))?;

    let router = proxy_router(state);
    tokio::spawn(async move {
        // ⚠️ axum::serve handles HTTP upgrades ⇒ ⇒ the local WebSocket
        // handshake works without any extra plumbing.
        if let Err(e) = axum::serve(listener, router).await {
            tracing::error!(error = %e, "零信任代理退出");
        }
    });

    Ok(addr)
}

/// Forward one request to the upstream and relay the answer.
async fn proxy_handler(State(state): State<ProxyState>, request: Request) -> Response {
    let (mut parts, body) = request.into_parts();

    let raw_path = parts
        .uri
        .path_and_query()
        .map(|p| p.as_str().to_string())
        .unwrap_or_else(|| "/".to_string());

    // ⚠️ **按服务名路由** ⇒ ⇒ 一个代理面对**多台**受信设备。
    let (connector, path) = match split_route(&raw_path, &state.routes) {
        RoutePick::Default(p) => (state.connector.clone(), p),
        RoutePick::Service(c, p) => (c, p),
        RoutePick::UnknownService(service) => {
            return (
                StatusCode::NOT_FOUND,
                format!("零信任代理：没有名为 '{service}' 的上游"),
            )
                .into_response();
        }
    };

    // ---- WebSocket? ⇒ upgrade path -------------------------------------
    let is_ws = parts
        .headers
        .get(UPGRADE)
        .and_then(|v| v.to_str().ok())
        .map(|v| v.eq_ignore_ascii_case("websocket"))
        .unwrap_or(false);

    if is_ws {
        let headers = forward_headers(&parts.headers, &state.default_headers, true);
        match WebSocketUpgrade::from_request_parts(&mut parts, &()).await {
            Ok(upgrade) => {
                let relay_state = state.clone();
                let ws_connector = connector.clone();
                return upgrade.on_upgrade(move |socket| {
                    relay_ws(socket, relay_state, path, headers, ws_connector)
                });
            }
            Err(_) => {
                // Not a valid handshake ⇒ fall through to plain HTTP.
            }
        }
    }

    // ---- plain HTTP ---------------------------------------------------
    let method = parts.method.clone();
    let headers = forward_headers(&parts.headers, &state.default_headers, false);

    let bytes = match axum::body::to_bytes(body, state.max_body_bytes).await {
        Ok(b) => b,
        Err(e) => {
            return (
                StatusCode::PAYLOAD_TOO_LARGE,
                format!("请求体读取失败或超出上限: {e}"),
            )
                .into_response();
        }
    };

    let io = match (connector)().await {
        Ok(io) => io,
        Err(e) => {
            return (
                StatusCode::BAD_GATEWAY,
                format!("拨上游失败: {e}"),
            )
                .into_response();
        }
    };

    let body = if bytes.is_empty() { None } else { Some(bytes) };

    let (head, incoming) = match request_over_stream_bytes(
        io,
        &state.upstream_host,
        method.as_str(),
        &path,
        body,
        Some(headers),
    )
    .await
    {
        Ok(pair) => pair,
        Err(e) => {
            return (
                StatusCode::BAD_GATEWAY,
                format!("上游请求失败: {e}"),
            )
                .into_response();
        }
    };

    // ⚠️ Drop hop-by-hop headers **and** content-length: the body is relayed
    // as a stream ⇒ ⇒ the local response is chunked ⇒ ⇒ content-length would
    // be a lie (and hyper would abort on a mismatch).
    let mut out_headers = HeaderMap::new();
    for (name, value) in head.headers {
        if is_hop_by_hop(&name) || name.eq_ignore_ascii_case("content-length") {
            continue;
        }
        if let (Ok(name), Ok(value)) = (
            HeaderName::from_bytes(name.as_bytes()),
            HeaderValue::from_str(&value),
        ) {
            out_headers.insert(name, value);
        }
    }

    // ⚠️ Frame-by-frame relay ⇒ ⇒ SSE chunks are **not** flattened.
    let stream = incoming.into_data_stream();

    let mut response = Response::new(Body::from_stream(stream));
    *response.status_mut() = head.status;
    *response.headers_mut() = out_headers;
    response
}

/// Relay frames between the local WebSocket and the upstream one.
async fn relay_ws(
    local: WebSocket,
    state: ProxyState,
    path: String,
    headers: Vec<(String, String)>,
    connector: Connector,
) {
    let io = match (connector)().await {
        Ok(io) => io,
        Err(e) => {
            tracing::warn!(error = %e, "零信任代理：拨上游失败（WS）");
            return;
        }
    };

    let (remote, _response) =
        match connect_ws_over_stream(io, &state.upstream_host, &path, headers).await {
            Ok(pair) => pair,
            Err(e) => {
                tracing::warn!(error = %e, "零信任代理：上游 WS 握手失败");
                return;
            }
        };

    let (mut local_sink, mut local_stream) = local.split();
    let (mut remote_sink, mut remote_stream) = remote.split();

    let upstream_task = tokio::spawn(async move {
        while let Some(frame) = local_stream.next().await {
            match frame {
                Ok(frame) => {
                    if remote_sink.send(to_tungstenite(frame)).await.is_err() {
                        break;
                    }
                }
                Err(_) => break,
            }
        }
    });

    let downstream_task = tokio::spawn(async move {
        while let Some(frame) = remote_stream.next().await {
            match frame {
                Ok(frame) => {
                    if let Some(frame) = from_tungstenite(frame) {
                        if local_sink.send(frame).await.is_err() {
                            break;
                        }
                    }
                }
                Err(_) => break,
            }
        }
    });

    let _ = tokio::join!(upstream_task, downstream_task);
}

/// axum's WebSocket message → tungstenite's.
///
/// ⚠️ They are **different types** (axum wraps tungstenite's) ⇒ ⇒ the
/// conversion is explicit, not a cast.
fn to_tungstenite(message: AxumMessage) -> ts::Message {
    match message {
        AxumMessage::Text(text) => ts::Message::Text(text.as_str().into()),
        AxumMessage::Binary(data) => ts::Message::Binary(data),
        AxumMessage::Ping(data) => ts::Message::Ping(data),
        AxumMessage::Pong(data) => ts::Message::Pong(data),
        AxumMessage::Close(None) => ts::Message::Close(None),
        AxumMessage::Close(Some(frame)) => ts::Message::Close(Some(ts::protocol::CloseFrame {
            code: ts::protocol::frame::coding::CloseCode::from(frame.code),
            reason: frame.reason.as_str().into(),
        })),
    }
}

/// tungstenite's WebSocket message → axum's.
fn from_tungstenite(message: ts::Message) -> Option<AxumMessage> {
    match message {
        ts::Message::Text(text) => Some(AxumMessage::Text(text.as_str().into())),
        ts::Message::Binary(data) => Some(AxumMessage::Binary(data)),
        ts::Message::Ping(data) => Some(AxumMessage::Ping(data)),
        ts::Message::Pong(data) => Some(AxumMessage::Pong(data)),
        ts::Message::Close(None) => Some(AxumMessage::Close(None)),
        ts::Message::Close(Some(frame)) => Some(AxumMessage::Close(Some(
            axum::extract::ws::CloseFrame {
                code: u16::from(frame.code),
                reason: frame.reason.as_str().into(),
            },
        ))),
        // Raw frames are an implementation detail ⇒ ⇒ drop them.
        ts::Message::Frame(_) => None,
    }
}

/// Headers to forward upstream.
///
/// ⚠️ **The proxy's own credentials WIN** —— the client may be carrying a
/// key for a **different** brain (the desktop's local key, for instance)
/// ⇒ ⇒ if both were forwarded, the upstream would read whichever came
/// first ⇒ ⇒ "the right key" and "the wrong key" become a race ⇒ ⇒
/// the proxy is the one that knows which brain it is talking to.
fn forward_headers(
    src: &HeaderMap,
    defaults: &[(String, String)],
    websocket: bool,
) -> Vec<(String, String)> {
    let owned: Vec<String> = defaults.iter().map(|(k, _)| k.to_ascii_lowercase()).collect();
    let mut out = Vec::new();
    for (name, value) in src.iter() {
        let lower = name.as_str();
        if is_hop_by_hop(lower)
            || lower.eq_ignore_ascii_case("content-length")
            || lower.eq_ignore_ascii_case("host")
        {
            continue;
        }
        if owned.iter().any(|k| k == lower) {
            // ⚠️ The proxy's own credential replaces the client's.
            continue;
        }
        if websocket && lower.starts_with("sec-websocket-") {
            // The proxy runs its **own** handshake on the upstream side ⇒ ⇒
            // the local client's key/version must not be replayed.
            continue;
        }
        if let Ok(value) = value.to_str() {
            out.push((lower.to_string(), value.to_string()));
        }
    }
    for (name, value) in defaults {
        out.push((name.clone(), value.clone()));
    }
    out
}

/// RFC 7230 hop-by-hop headers —— never forwarded.
fn is_hop_by_hop(name: &str) -> bool {
    matches!(
        name.to_ascii_lowercase().as_str(),
        "connection"
            | "keep-alive"
            | "proxy-authenticate"
            | "proxy-authorization"
            | "te"
            | "trailer"
            | "transfer-encoding"
            | "upgrade"
    )
}

#[cfg(test)]
mod tests {
    //! **Runtime evidence for the proxy** —— the production code path, driven
    //! over plain TCP.
    //!
    //! ⚠️ The upstream here is a **plain TCP server** running the very same
    //! `serve_ziti_stream` the ziti dark service uses ⇒ ⇒ the only thing
    //! swapped is where the stream comes from ⇒ ⇒ the proxy logic itself is
    //! what gets proven.

    use super::*;
    use axum::extract::ws::{Message as AxumMessage, WebSocket, WebSocketUpgrade};
    use axum::response::sse::{Event, Sse};
    use axum::routing::{get, post};
    use futures::SinkExt;
    use std::convert::Infallible;
    use std::time::{Duration, Instant};

    /// Start an "upstream" on plain TCP.
    async fn spawn_upstream(app: axum::Router) -> u16 {
        let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0))
            .await
            .expect("bind");
        let port = listener.local_addr().expect("addr").port();
        tokio::spawn(async move {
            while let Ok((stream, _)) = listener.accept().await {
                let app = app.clone();
                tokio::spawn(async move {
                    crate::transport::ziti::dark_service::serve_ziti_stream(stream, app).await;
                });
            }
        });
        port
    }

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

    async fn echo_ws(ws: WebSocketUpgrade) -> impl axum::response::IntoResponse {
        ws.on_upgrade(|mut socket: WebSocket| async move {
            while let Some(Ok(AxumMessage::Text(text))) = socket.recv().await {
                if socket
                    .send(AxumMessage::Text(format!("echo:{text}").into()))
                    .await
                    .is_err()
                {
                    break;
                }
            }
        })
    }

    /// ✅ A plain request/response survives the trip, **including the
    /// credential the proxy injects**.
    #[tokio::test]
    async fn forwards_request_response_and_injected_credentials() {
        let app = axum::Router::new().route(
            "/api/echo",
            post(|headers: axum::http::HeaderMap, body: axum::body::Bytes| async move {
                let key = headers
                    .get("x-shadow-key")
                    .and_then(|v| v.to_str().ok())
                    .unwrap_or("")
                    .to_string();
                format!("{key}|{}", String::from_utf8_lossy(&body))
            }),
        );
        let upstream_port = spawn_upstream(app).await;

        let state = ProxyState::new(tcp_connector(upstream_port), "brain.local")
            .with_default_headers(vec![("X-Shadow-Key".to_string(), "proxy-key".to_string())]);
        let proxy_addr = spawn_proxy(state, "127.0.0.1:0".parse().expect("addr"))
            .await
            .expect("proxy");

        let response = reqwest::Client::new()
            .post(format!("http://{proxy_addr}/api/echo"))
            // ⚠️ 客户端**带着自己的钥匙**（桌面端本地那把）⇒ ⇒ 代理配的那把
            // 应当**盖掉**它 ⇒ ⇒ 否则上游读到哪一把就成了顺序问题。
            .header("X-Shadow-Key", "client-key")
            .body("ping")
            .send()
            .await
            .expect("请求");

        assert_eq!(response.status(), 200);
        let body = response.text().await.expect("body");
        assert_eq!(
            body, "proxy-key|ping",
            "代理配的凭据应当**盖掉**客户端那把，并且请求体原样到达上游"
        );
    }

    /// ✅ **SSE stays chunked** through the proxy.
    ///
    /// ⚠️ The judgement is the **gap between two chunks** ⇒ ⇒ a buffering
    /// proxy makes both arrive together ⇒ ⇒ the gap is what distinguishes
    /// "streamed" from "collected".
    #[tokio::test]
    async fn keeps_sse_chunks_apart() {
        let app = axum::Router::new().route("/sse", get(sse_handler));
        let upstream_port = spawn_upstream(app).await;

        let state = ProxyState::new(tcp_connector(upstream_port), "brain.local");
        let proxy_addr = spawn_proxy(state, "127.0.0.1:0".parse().expect("addr"))
            .await
            .expect("proxy");

        let response = reqwest::Client::new()
            .get(format!("http://{proxy_addr}/sse"))
            .send()
            .await
            .expect("请求");
        assert_eq!(response.status(), 200);
        assert_eq!(
            response
                .headers()
                .get("content-type")
                .and_then(|v| v.to_str().ok()),
            Some("text/event-stream"),
            "上游的 content-type 应当透传"
        );

        let mut stream = response.bytes_stream();
        let started = Instant::now();

        let first = tokio::time::timeout(Duration::from_secs(5), async {
            use futures::StreamExt;
            stream.next().await
        })
        .await
        .expect("第一块应当到达")
        .expect("应当有块")
        .expect("块读取成功");
        let first_at = started.elapsed();

        let second = tokio::time::timeout(Duration::from_secs(5), async {
            use futures::StreamExt;
            stream.next().await
        })
        .await
        .expect("第二块应当到达")
        .expect("应当有块")
        .expect("块读取成功");
        let gap = started.elapsed() - first_at;

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
            "代理把 SSE 压平了（缓冲）—— 实测间隔 {gap:?}"
        );
    }

    /// ✅ A WebSocket survives the trip **in both directions**.
    #[tokio::test]
    async fn relays_websocket_frames_both_ways() {
        let app = axum::Router::new().route("/ws", get(echo_ws));
        let upstream_port = spawn_upstream(app).await;

        let state = ProxyState::new(tcp_connector(upstream_port), "brain.local");
        let proxy_addr = spawn_proxy(state, "127.0.0.1:0".parse().expect("addr"))
            .await
            .expect("proxy");

        let (mut socket, response) = tokio_tungstenite::connect_async(format!(
            "ws://{proxy_addr}/ws"
        ))
        .await
        .expect("握手");
        assert_eq!(response.status().as_u16(), 101);

        socket
            .send(ts::Message::Text("hi".into()))
            .await
            .expect("发送");

        let reply = tokio::time::timeout(Duration::from_secs(5), socket.next())
            .await
            .expect("回包应当到达")
            .expect("应当有帧")
            .expect("帧读取成功");

        match reply {
            ts::Message::Text(text) => assert_eq!(text.as_str(), "echo:hi"),
            other => panic!("回包不是文本帧：{other:?}"),
        }
    }

    /// ⚠️ **按服务名路由** —— 一个代理面对**两台**受信设备。
    ///
    /// ⇒ ⇒ 判据是「请求**真的到了那台设备**」（两台回的内容不同）
    /// ⇒ ⇒ 不是「路由表写进去了」。
    #[tokio::test]
    async fn a_service_prefixed_path_reaches_that_service() {
        let app_a = axum::Router::new().fallback(any(|req: Request| async move {
            format!("device-a:{}", req.uri().path())
        }));
        let app_b = axum::Router::new().fallback(any(|req: Request| async move {
            format!("device-b:{}", req.uri().path())
        }));
        let port_a = spawn_upstream(app_a).await;
        let port_b = spawn_upstream(app_b).await;

        let state = ProxyState::new(tcp_connector(port_a), "device-a")
            .with_route("b", tcp_connector(port_b));
        let addr = spawn_proxy(state, "127.0.0.1:0".parse().expect("bind"))
            .await
            .expect("代理应当起得来");

        let client = reqwest::Client::new();

        // 默认路径 ⇒ 默认上游。
        let body = client
            .get(format!("http://{addr}/x"))
            .send()
            .await
            .expect("请求应当通")
            .text()
            .await
            .expect("body");
        assert_eq!(body, "device-a:/x");

        // 服务前缀 ⇒ 对应那台设备，路径去掉前缀。
        let body = client
            .get(format!("http://{addr}/_zt/b/x"))
            .send()
            .await
            .expect("请求应当通")
            .text()
            .await
            .expect("body");
        assert_eq!(body, "device-b:/x");

        // 认不出的服务 ⇒ ⇒ **明确 404**（绝不静默落到默认设备）。
        let status = client
            .get(format!("http://{addr}/_zt/nope/x"))
            .send()
            .await
            .expect("请求应当通")
            .status();
        assert_eq!(status.as_u16(), 404);
    }
}
