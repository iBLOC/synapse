//! WebSocket over a class-TCP stream —— the **client** half of the zero-trust upgrade.
//!
//! ## Why this file exists
//!
//! The server half (`super::dark_service::serve_ziti_stream`) already opened
//! `.with_upgrades()` ⇒ the brain can *accept* WebSocket over a dark service.
//! ⇒ ⇒ But the **client** side was empty: the ziti client in this crate only
//! had request/response and SSE ⇒ ⇒ **nothing could put a WebSocket on a ziti
//! stream** ⇒ ⇒ the ESP32 signalling channel
//! (`/api/webrtc/{modality}/sessions/{id}/ws`) could not be driven even
//! though the server was ready for it.
//!
//! ## ⚠️ IO is generic again —— same reason as everywhere else in this module
//!
//! `ZitiStream` is "a duplex stream that behaves like TCP" ⇒ ⇒ a plain
//! `TcpStream` satisfies it too ⇒ ⇒ the **same** code path can be **really
//! run** on loopback (see the tests at the bottom of this file) ⇒ ⇒ "this
//! carrier supports WebSocket" has **runtime evidence**, not just "it compiles".

use std::sync::Arc;
use std::time::Duration;

use futures::{SinkExt, StreamExt};
use tokio::io::{AsyncRead, AsyncWrite};
use tokio_tungstenite::tungstenite::handshake::client::generate_key;
use tokio_tungstenite::tungstenite::http::{Request, Response, Uri};
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::{client_async, WebSocketStream};
use super::ZitiStream;

use super::gateway::ZitiGateway;
use super::ZitiError;

/// Complete a WebSocket handshake **over any duplex stream**.
///
/// Returns the socket plus the peer's handshake response (`101 Switching
/// Protocols`).
///
/// ⚠️ The request is built **by hand** on purpose: tungstenite's
/// `IntoClientRequest for http::Request<()>` passes the request through
/// **without touching the headers** ⇒ ⇒ we must supply `Connection`,
/// `Upgrade`, `Sec-WebSocket-Version` and `Sec-WebSocket-Key` ourselves,
/// otherwise the peer rejects the handshake.
///
/// ⚠️ `http` is imported from **tungstenite's** re-export, not from
/// `hyper`'s —— they are the same `http` 1.x crate in practice, but going
/// through tungstenite's re-export makes the trait impl match **by
/// construction** instead of by luck.
pub async fn connect_ws_over_stream<S>(
    io: S,
    host: &str,
    path: &str,
    headers: Vec<(String, String)>,
) -> Result<(WebSocketStream<S>, Response<Option<Vec<u8>>>), ZitiError>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    let uri: Uri = format!("ws://{host}{path}")
        .parse()
        .map_err(|e| ZitiError::ConnectionError(format!("WS URI 解析失败: {e}")))?;

    let mut builder = Request::builder()
        .method("GET")
        .uri(uri)
        .header("Host", host)
        .header("Connection", "Upgrade")
        .header("Upgrade", "websocket")
        .header("Sec-WebSocket-Version", "13")
        .header("Sec-WebSocket-Key", generate_key());

    for (key, value) in headers {
        builder = builder.header(&key, &value);
    }

    let request = builder
        .body(())
        .map_err(|e| ZitiError::ConnectionError(format!("WS 请求构造失败: {e}")))?;

    let (socket, response) = client_async(request, io)
        .await
        .map_err(|e| ZitiError::ConnectionError(format!("WebSocket 握手失败: {e}")))?;

    Ok((socket, response))
}

/// Zero-trust WebSocket client —— dial, handshake, then exchange frames.
///
/// ⚠️ The timeout covers **dial ＋ handshake only** ⇒ ⇒ it deliberately does
/// **not** cover the frame exchange: a WebSocket is a long-lived connection
/// and a read timeout on it would kill the connection, not "time out a
/// request".
pub struct ZitiWebSocketClient {
    gateway: Arc<ZitiGateway>,
    timeout: Duration,
}

impl ZitiWebSocketClient {
    pub fn new(gateway: Arc<ZitiGateway>) -> Self {
        Self {
            gateway,
            timeout: Duration::from_secs(30),
        }
    }

    pub fn with_timeout(gateway: Arc<ZitiGateway>, timeout: Duration) -> Self {
        Self { gateway, timeout }
    }

    /// Dial `service_name` and open a WebSocket to `path` on it.
    pub async fn connect(
        &self,
        service_name: &str,
        path: &str,
        headers: Vec<(String, String)>,
    ) -> Result<ZitiWebSocket, ZitiError> {
        let stream = tokio::time::timeout(self.timeout, self.gateway.dial(service_name))
            .await
            .map_err(|_| ZitiError::ConnectionError(format!("拨号 {service_name} 超时")))?
            .map_err(|e| ZitiError::ConnectionError(e.to_string()))?;

        let (socket, _response) = tokio::time::timeout(
            self.timeout,
            connect_ws_over_stream(stream, service_name, path, headers),
        )
        .await
        .map_err(|_| ZitiError::ConnectionError("WebSocket 握手超时".to_string()))??;

        Ok(ZitiWebSocket { inner: socket })
    }
}

/// A WebSocket that lives on a ziti stream.
pub struct ZitiWebSocket {
    inner: WebSocketStream<ZitiStream>,
}

impl ZitiWebSocket {
    /// Send one text frame.
    pub async fn send_text(&mut self, text: &str) -> Result<(), ZitiError> {
        self.inner
            .send(Message::Text(text.into()))
            .await
            .map_err(|e| ZitiError::IoError(std::io::Error::other(e.to_string())))
    }

    /// Send one binary frame.
    pub async fn send_binary(&mut self, data: &[u8]) -> Result<(), ZitiError> {
        self.inner
            .send(Message::Binary(data.to_vec().into()))
            .await
            .map_err(|e| ZitiError::IoError(std::io::Error::other(e.to_string())))
    }

    /// Read the next **data** frame (text or binary) as a string.
    ///
    /// ⚠️ Control frames (ping/pong/close) are handled by the protocol layer
    /// ⇒ ⇒ they never surface here ⇒ ⇒ this only returns when there is
    /// actual payload.
    pub async fn recv_text(&mut self) -> Result<String, ZitiError> {
        loop {
            match self.inner.next().await {
                Some(Ok(Message::Text(text))) => return Ok(text.to_string()),
                Some(Ok(Message::Binary(data))) => {
                    return Ok(String::from_utf8_lossy(&data).to_string());
                }
                Some(Ok(_)) => continue,
                Some(Err(e)) => {
                    return Err(ZitiError::IoError(std::io::Error::other(e.to_string())));
                }
                None => {
                    return Err(ZitiError::ConnectionError(
                        "上游关闭了 WebSocket".to_string(),
                    ));
                }
            }
        }
    }

    /// Close the socket politely.
    pub async fn close(&mut self) -> Result<(), ZitiError> {
        self.inner
            .close(None)
            .await
            .map_err(|e| ZitiError::IoError(std::io::Error::other(e.to_string())))
    }
}

#[cfg(test)]
mod tests {
    //! **Runtime evidence for the client half** —— the production code path,
    //! driven over a plain TCP stream.

    use super::*;
    use axum::extract::ws::{Message as AxumMessage, WebSocket, WebSocketUpgrade};
    use axum::routing::get;
    use futures::SinkExt;
    use std::time::Duration;

    async fn echo_ws(ws: WebSocketUpgrade) -> impl axum::response::IntoResponse {
        ws.on_upgrade(|mut socket: WebSocket| async move {
            if let Some(Ok(AxumMessage::Text(text))) = socket.recv().await {
                let _ = socket.send(AxumMessage::Text(format!("echo:{text}").into())).await;
            }
        })
    }

    /// ✅ A WebSocket handshake ＋ one frame round-trip **really happens** on a
    /// class-TCP stream.
    ///
    /// ⚠️ This is the **same** `connect_ws_over_stream` the ziti client uses
    /// ⇒ ⇒ the only thing swapped is where the stream comes from.
    #[tokio::test]
    async fn websocket_roundtrip_over_a_plain_stream() {
        let app = axum::Router::new().route("/ws", get(echo_ws));
        let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0))
            .await
            .expect("bind");
        let port = listener.local_addr().expect("addr").port();
        tokio::spawn(async move {
            while let Ok((stream, _)) = listener.accept().await {
                let app = app.clone();
                tokio::spawn(async move {
                    super::super::dark_service::serve_ziti_stream(stream, app).await;
                });
            }
        });

        let stream = tokio::net::TcpStream::connect(("127.0.0.1", port))
            .await
            .expect("connect");

        let (mut socket, response) =
            connect_ws_over_stream(stream, &format!("127.0.0.1:{port}"), "/ws", Vec::new())
                .await
                .expect("握手");

        assert_eq!(
            response.status().as_u16(),
            101,
            "应当是 101 Switching Protocols"
        );

        socket.send(Message::Text("hello".into())).await.expect("发送");

        let reply = tokio::time::timeout(Duration::from_secs(5), socket.next())
            .await
            .expect("回包应当到达")
            .expect("应当有帧")
            .expect("帧读取成功");

        match reply {
            Message::Text(text) => assert_eq!(text.as_str(), "echo:hello"),
            other => panic!("回包不是文本帧：{other:?}"),
        }
    }
}
