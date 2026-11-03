//! HTTP client that goes through Ziti instead of the public internet.
//!
//! Use when you need to talk to a peer encephalon / tiangong service
//! whose service name only resolves inside the Ziti overlay.
//!
//! ## ⚠️ 两条读法，一种缓冲、一种不缓冲
//!
//! | API | body | 适合 |
//! |---|---|---|
//! | [`HttpResponse`] | String（**整个缓冲**） | 普通请求/响应 |
//! | [`ZitiHttpClient::request_stream`] | Incoming（**不缓冲**） | **SSE / 分块响应** |
//!
//! ⇒ ⇒ **String 这个形状装不下流** ⇒ ⇒ 拿对话流只能走流式那条。

use anyhow::Result;
use bytes::Bytes;
use http_body_util::{BodyExt, Full};
use hyper::body::Incoming;
use hyper::{Request, StatusCode};
use hyper_util::rt::TokioIo;
use std::sync::Arc;
use std::time::Duration;

use super::gateway::ZitiGateway;

/// 响应**头**（不含 body）。
#[derive(Debug, Clone)]
pub struct HttpResponseHead {
    pub status: StatusCode,
    pub headers: Vec<(String, String)>,
}

/// 整个 body 都被读进内存的响应。
///
/// ⚠️ **流式语义在这里被压平** —— SSE 的分块会被拼成一整块
/// ⇒ ⇒ 要拿流，用 [`ZitiHttpClient::request_stream`]。
#[derive(Debug, Clone)]
pub struct HttpResponse {
    pub status: StatusCode,
    pub headers: Vec<(String, String)>,
    pub body: String,
}

pub struct ZitiHttpClient {
    gateway: Arc<ZitiGateway>,
    timeout: Duration,
}

impl ZitiHttpClient {
    pub fn new(gateway: Arc<ZitiGateway>) -> Self {
        Self {
            gateway,
            timeout: Duration::from_secs(30),
        }
    }

    pub fn with_timeout(gateway: Arc<ZitiGateway>, timeout: Duration) -> Self {
        Self { gateway, timeout }
    }

    pub async fn get(&self, service_name: &str, path: &str) -> Result<HttpResponse> {
        self.request(service_name, "GET", path, None, None).await
    }

    pub async fn post(
        &self,
        service_name: &str,
        path: &str,
        body: &str,
    ) -> Result<HttpResponse> {
        self.request(service_name, "POST", path, Some(body), None).await
    }

    pub async fn put(
        &self,
        service_name: &str,
        path: &str,
        body: &str,
    ) -> Result<HttpResponse> {
        self.request(service_name, "PUT", path, Some(body), None).await
    }

    pub async fn delete(&self, service_name: &str, path: &str) -> Result<HttpResponse> {
        self.request(service_name, "DELETE", path, None, None).await
    }

    pub async fn patch(
        &self,
        service_name: &str,
        path: &str,
        body: &str,
    ) -> Result<HttpResponse> {
        self.request(service_name, "PATCH", path, Some(body), None).await
    }
    /// ⚠️ **流式**请求 —— body **不缓冲** ⇒ ⇒ SSE 的流式语义保留。
    ///
    /// ⚠️ **timeout 只管「握手 ＋ 发出请求」** ⇒ ⇒ **不管 body 的读取**
    /// ⇒ ⇒ 这是刻意的：SSE / WebSocket 那种长连接**本来就要一直读**
    /// ⇒ ⇒ 用 30s 超时去卡它，等于**把流式掐死**。
    ///
    /// ⚠️ 返回的 Incoming 依附于后台那条连接 ⇒ ⇒ 调用方**必须把它读完
    /// 或 drop 掉**（drop 会让 hyper 关连接）⇒ ⇒ 别把它丢在那儿不管。
    pub async fn request_stream(
        &self,
        service_name: &str,
        method: &str,
        path: &str,
        body: Option<&str>,
        headers: Option<Vec<(String, String)>>,
    ) -> Result<(HttpResponseHead, Incoming)> {
        let stream =
            tokio::time::timeout(self.timeout, self.gateway.dial(service_name)).await??;
        let (head, incoming) = tokio::time::timeout(
            self.timeout,
            request_over_stream(stream, service_name, method, path, body, headers),
        )
        .await??;
        Ok((head, incoming))
    }

    /// 普通请求/响应 —— 整个 body 读成一个 String。
    pub async fn request(
        &self,
        service_name: &str,
        method: &str,
        path: &str,
        body: Option<&str>,
        headers: Option<Vec<(String, String)>>,
    ) -> Result<HttpResponse> {
        let (head, incoming) = self
            .request_stream(service_name, method, path, body, headers)
            .await?;
        let collected = incoming.collect().await?;
        Ok(HttpResponse {
            status: head.status,
            headers: head.headers,
            body: String::from_utf8(collected.to_bytes().to_vec())?,
        })
    }
}
/// 在**任意一条类 TCP 流**上发一个 HTTP 请求 —— 返回**不缓冲**的响应。
///
/// ## ⚠️ 为什么 IO 是泛型的
///
/// 「流从哪来」与「HTTP 怎么读」**解耦** ⇒ ⇒ 于是同一条代码路径可以在
/// loopback TCP 上**真跑**（本文件的测试就这么验）⇒ ⇒ 「SSE 在这条承载
/// 上是流式的」这句话**有运行时证据**，而不只是「编译过」。
///
/// ⚠️ 与 serve_ziti_stream 是**同一个思路**：承载面的能力**必须能在没有
/// ziti 网络的地方被验证**。
pub async fn request_over_stream<S>(
    io: S,
    host: &str,
    method: &str,
    path: &str,
    body: Option<&str>,
    headers: Option<Vec<(String, String)>>,
) -> Result<(HttpResponseHead, Incoming)>
where
    S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin + Send + 'static,
{
    // ⚠️ 文本版**只**多干一件事：给有 body 的请求补上 JSON 的 Content-Type
    // ⇒ ⇒ 语义与旧实现一致（用户头在前，Content-Type 追加在后）。
    let (headers, body) = match body {
        Some(text) => {
            let mut headers = headers.unwrap_or_default();
            headers.push((
                "Content-Type".to_string(),
                "application/json".to_string(),
            ));
            (Some(headers), Some(Bytes::from(text.to_string())))
        }
        None => (headers, None),
    };
    request_over_stream_bytes(io, host, method, path, body, headers).await
}

/// 在**任意一条类 TCP 流**上发一个 HTTP 请求 —— body 是**字节**。
///
/// ⚠️ 与 `request_over_stream` 的区别：**不碰 Content-Type** ⇒ ⇒ 代理
/// 转发时必须用它（否则二进制请求体被贴上 JSON 标签）。
/// ⚠️ Content-Length 由**本函数**按实际长度算 ⇒ ⇒ 调用方不用管。
pub async fn request_over_stream_bytes<S>(
    io: S,
    host: &str,
    method: &str,
    path: &str,
    body: Option<Bytes>,
    headers: Option<Vec<(String, String)>>,
) -> Result<(HttpResponseHead, Incoming)>
where
    S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin + Send + 'static,
{
    let io = TokioIo::new(io);
    let (mut sender, conn) = hyper::client::conn::http1::handshake(io).await?;

    // ⚠️ 连接在后台跑 ⇒ ⇒ Incoming 读的时候它还活着 ⇒ ⇒ 流式读得到。
    tokio::spawn(async move {
        if let Err(e) = conn.await {
            tracing::error!("Connection error: {}", e);
        }
    });

    let mut req_builder = Request::builder()
        .method(method)
        .uri(path)
        .header("Host", host);

    if let Some(headers) = headers {
        for (key, value) in headers {
            req_builder = req_builder.header(&key, &value);
        }
    }

    let req = if let Some(bytes) = body {
        req_builder
            .header("Content-Length", bytes.len())
            .body(Full::new(bytes))?
    } else {
        req_builder.body(Full::new(Bytes::new()))?
    };

    let resp = sender.send_request(req).await?;
    let status = resp.status();
    let headers = resp
        .headers()
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("").to_string()))
        .collect();
    Ok((HttpResponseHead { status, headers }, resp.into_body()))
}

#[cfg(test)]
mod tests {
    //! **客户端侧的运行时实证** —— 同一段「读 HTTP」的代码，换成普通 TCP。
    //!
    //! ⚠️ **它钉的是「谁在缓冲」** —— hyper 的 client conn **本来就**是流式
    //! 的（`Incoming` 逐帧给）⇒ ⇒ 真正把流压平的是**我们自己的 DTO**
    //! （`HttpResponse { body: String }`）⇒ ⇒ 所以「加一个不缓冲的 API」
    //! 就够了，**不用**改 hyper。

    use super::*;
    use axum::response::sse::{Event, Sse};
    use axum::routing::get;
    use http_body_util::BodyExt;
    use std::convert::Infallible;
    use std::time::{Duration, Instant};
    use tokio::net::{TcpListener, TcpStream};

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
    /// ✅ **客户端读到的 SSE 是分块的** —— 不是等整个响应收完。
    ///
    /// ⚠️ 判据是「**两块之间隔 ≥80ms**」⇒ ⇒ 整体缓冲时两块会**同时**到达
    /// ⇒ ⇒ 只有间隔才能区分。
    #[tokio::test]
    async fn response_body_is_read_as_a_stream() {
        let app = axum::Router::new().route("/sse", get(sse_handler));
        let listener = TcpListener::bind(("127.0.0.1", 0)).await.expect("bind");
        let port = listener.local_addr().expect("addr").port();
        tokio::spawn(async move {
            while let Ok((stream, _)) = listener.accept().await {
                let app = app.clone();
                tokio::spawn(async move {
                    super::super::dark_service::serve_ziti_stream(stream, app).await;
                });
            }
        });

        let stream = TcpStream::connect(("127.0.0.1", port)).await.expect("connect");
        let (head, mut body) = request_over_stream(
            stream,
            &format!("127.0.0.1:{port}"),
            "GET",
            "/sse",
            None,
            None,
        )
        .await
        .expect("请求");
        assert_eq!(head.status, 200);

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
            "客户端读到的两块之间应隔 ≥80ms（流式）；实测 {gap:?}"
        );
    }
}
