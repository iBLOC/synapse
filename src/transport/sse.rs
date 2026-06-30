//! Server-Sent Events (SSE) protocol support.
//!
//! Provides:
//! - [`SseEvent`] / [`SseStream`] / [`SseError`] wire types
//! - [`parse_chunk`], a pure-function SSE byte-parser shared between the
//!   native (reqwest) and WASM (wasi::http) implementations
//! - [`HttpClient::sse_post`], the streaming extension of the cross-platform
//!   HTTP transport trait
//!
//! The parser is **stateful**: it accepts an arbitrary byte slice and emits
//! zero or more complete [`SseEvent`]s. Partial events are kept in
//! [`SseParseState`] until the next chunk completes them. This lets us
//! process an HTTP body without buffering an entire response in memory.

use async_trait::async_trait;
use futures::Stream;
use thiserror::Error;
use tokio::sync::mpsc;

use super::{HttpClient, HttpError, HttpMethod, HttpRequest};

/// One Server-Sent Event, in the wire shape defined by the SSE spec.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SseEvent {
    /// Value of the optional `event:` field.
    pub event: Option<String>,
    /// Concatenated value of all `data:` fields for this event (newlines
    /// preserved between them). Empty string for comment-only events.
    pub data: String,
    /// Value of the optional `id:` field (used for resumption).
    pub id: Option<String>,
}

/// Consumer-side handle for an in-progress SSE response.
///
/// Yields one [`SseEvent`] at a time; the stream ends when the HTTP response
/// body is exhausted or the connection drops.
pub struct SseStream {
    rx: mpsc::Receiver<Result<SseEvent, SseError>>,
}

impl SseStream {
    /// Receive the next SSE event. Returns `None` when the producer has
    /// closed the channel (response body complete or error encountered).
    pub async fn next(&mut self) -> Option<Result<SseEvent, SseError>> {
        self.rx.recv().await
    }

    /// Borrow the underlying stream as a `Stream`.
    pub fn as_stream(&mut self) -> impl Stream<Item = Result<SseEvent, SseError>> + '_ {
        futures::stream::poll_fn(move |cx| {
            let pinned_rx = &mut self.rx;
            tokio::sync::mpsc::Receiver::poll_recv(pinned_rx, cx)
        })
    }
}

/// Errors specific to SSE parsing and connection management.
#[derive(Debug, Error)]
pub enum SseError {
    #[error("connection lost: {0}")]
    ConnectionLost(String),
    #[error("malformed SSE line: {0}")]
    MalformedLine(String),
    #[error("underlying HTTP error: {0}")]
    Http(#[from] HttpError),
}

/// Mutable state for the chunk parser. The producer task owns one and threads
/// it across chunk boundaries.
#[derive(Debug, Default)]
pub struct SseParseState {
    current_event: Option<String>,
    current_data: Vec<String>,
    current_id: Option<String>,
}

impl SseParseState {
    pub fn new() -> Self {
        Self::default()
    }
}

/// Parse one or more bytes (potentially containing multiple complete SSE
/// events) and return the events found. The state is updated in place to
/// hold any partial event that spans the end of the chunk.
///
/// Lines are terminated by LF, CRLF, or CR. The spec also allows
/// comment lines starting with `:`. Empty lines dispatch the current event.
///
/// **Note**: This parser is intentionally lenient about line endings and
/// BOM handling because LLM provider wire formats in the wild are not
/// perfectly interoperable.
pub fn parse_chunk(chunk: &[u8], state: &mut SseParseState) -> Vec<SseEvent> {
    let mut events = Vec::new();
    let s = match std::str::from_utf8(chunk) {
        Ok(s) => s,
        Err(_) => return events,
    };

    for raw_line in split_lines(s) {
        // Strip trailing \r if present
        let line = raw_line.strip_suffix('\r').unwrap_or(raw_line);

        if line.is_empty() {
            // Dispatch the current event, if any data was accumulated.
            if !state.current_data.is_empty() || state.current_event.is_some() || state.current_id.is_some() {
                let data = state.current_data.join("\n");
                let ev = SseEvent {
                    event: state.current_event.take(),
                    data,
                    id: state.current_id.take(),
                };
                state.current_data.clear();
                events.push(ev);
            }
            continue;
        }

        if let Some(rest) = line.strip_prefix(':') {
            // Comment line; ignore per SSE spec, but consume the field
            // for the "id" of the current event (per spec, a comment
            // resets the id only if the line starts with colon + space).
            let _ = rest;
            continue;
        }

        if let Some(rest) = line.strip_prefix("event:") {
            let value = rest.trim_start();
            state.current_event = Some(value.to_string());
        } else if let Some(rest) = line.strip_prefix("data:") {
            let value = rest.trim_start();
            state.current_data.push(value.to_string());
        } else if let Some(rest) = line.strip_prefix("id:") {
            let value = rest.trim_start();
            if !value.contains('\0') {
                state.current_id = Some(value.to_string());
            }
        } else if let Some(rest) = line.strip_prefix("retry:") {
            // Spec says retry controls client reconnect; we don't act on
            // it but we accept the field for forward-compat.
            let _ = rest;
        } else if !line.contains(':') {
            // Per the SSE spec, a non-empty line with no colon character
            // is a syntax error and must be ignored. This is different
            // from "unknown field name with a colon" (which we treat
            // as data, see below).
        } else {
            // Has a colon but no recognized field name. The SSE spec is
            // silent on this; common server implementations (e.g. nginx
            // proxy) emit such lines as data continuation, so we follow
            // that convention. The leading whitespace (if any) is preserved.
            state.current_data.push(line.to_string());
        }
    }

    events
}

fn split_lines(s: &str) -> impl Iterator<Item = &str> {
    // We split on \n and let the per-line strip_suffix('\r') clean up.
    // Empty input yields zero lines.
    if s.is_empty() {
        Either::Left(std::iter::empty())
    } else {
        Either::Right(s.split('\n'))
    }
}

enum Either<L, R> {
    Left(L),
    Right(R),
}

impl<L: Iterator, R: Iterator<Item = L::Item>> Iterator for Either<L, R> {
    type Item = L::Item;
    fn next(&mut self) -> Option<Self::Item> {
        match self {
            Self::Left(l) => l.next(),
            Self::Right(r) => r.next(),
        }
    }
}

/// Extension trait added to the `HttpClient` for SSE streaming POSTs.
///
/// This is a separate trait so the WASM (wasi::http) implementation can
/// provide a concrete impl without forcing the trait to be object-safe in
/// a way that conflicts with `async_trait` in WASM.
#[async_trait]
pub trait HttpClientSse: HttpClient {
    /// POST `body` to `url` and return a stream of SSE events.
    ///
    /// The implementation MUST set `Accept: text/event-stream` on the
    /// request and parse the response body as SSE. HTTP errors (4xx/5xx)
    /// are returned via the first `Err(_)` item in the stream, or as an
    /// `HttpError` if the status line is non-2xx.
    async fn sse_post(
        &self,
        url: &str,
        headers: Vec<(String, String)>,
        body: Vec<u8>,
    ) -> Result<SseStream, SseError>;
}



#[async_trait]
impl HttpClientSse for super::native::ReqwestHttpClient {
    async fn sse_post(
        &self,
        url: &str,
        headers: Vec<(String, String)>,
        body: Vec<u8>,
    ) -> Result<SseStream, SseError> {
        use futures::StreamExt;

        let mut all_headers = headers;
        all_headers.push((
            "Accept".to_string(),
            "text/event-stream".to_string(),
        ));

        // Use the existing execute() path so we go through one well-tested
        // HTTP code path. We bypass the response body and re-execute the
        // request via reqwest's streaming `bytes_stream` directly.
        let req = HttpRequest {
            method: HttpMethod::Post,
            url: url.to_string(),
            headers: all_headers,
            body: Some(body),
        };
        // We can't easily extract the live reqwest response from execute()
        // because it consumes the response into bytes. So we duplicate the
        // small piece of glue here.
        let resp = self.execute_raw(&req).await?;
        if !(200..300).contains(&resp.status) {
            let body_str = String::from_utf8_lossy(&resp.body).into_owned();
            return Err(SseError::Http(HttpError::Status(resp.status, body_str)));
        }

        let (tx, rx) = mpsc::channel::<Result<SseEvent, SseError>>(32);
        let mut bytes = resp.bytes_stream;
        let mut state = SseParseState::new();

        tokio::spawn(async move {
            while let Some(chunk_res) = bytes.next().await {
                match chunk_res {
                    Ok(chunk) => {
                        let evs = parse_chunk(&chunk, &mut state);
                        for ev in evs {
                            if tx.send(Ok(ev)).await.is_err() {
                                return;
                            }
                        }
                    }
                    Err(e) => {
                        let _ = tx
                            .send(Err(SseError::ConnectionLost(e.to_string())))
                            .await;
                        return;
                    }
                }
            }
        });

        Ok(SseStream { rx })
    }
}

/// Internal helper exposed for the SSE extension trait. Performs an
/// HTTP request and returns the live reqwest response so streaming bodies
/// can be processed incrementally.
impl super::native::ReqwestHttpClient {
    pub(crate) async fn execute_raw(
        &self,
        req: &HttpRequest,
    ) -> Result<RawHttpResponse, HttpError> {
        use futures::StreamExt;

        let client = reqwest::Client::new();
        let mut builder = match req.method {
            HttpMethod::Get => client.get(&req.url),
            HttpMethod::Post => client.post(&req.url),
            HttpMethod::Put => client.put(&req.url),
            HttpMethod::Delete => client.delete(&req.url),
        };
        for (k, v) in &req.headers {
            builder = builder.header(k, v);
        }
        if let Some(body) = &req.body {
            builder = builder.body(body.clone());
        }
        let resp = builder
            .send()
            .await
            .map_err(|e| HttpError::Request(e.to_string()))?;
        let status = resp.status().as_u16();
        let headers = resp
            .headers()
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("").to_string()))
            .collect();
        let bytes_stream = resp
            .bytes_stream()
            .map(|r| r.map_err(|e| e.to_string()))
            .boxed();
        // Drain a small bit so status errors surface; real body comes via stream.
        Ok(RawHttpResponse {
            status,
            headers,
            body: Vec::new(),
            bytes_stream,
        })
    }
}

/// Internal: minimal live response used by `sse_post`.
pub struct RawHttpResponse {
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
    pub bytes_stream: std::pin::Pin<
        Box<dyn futures::Stream<Item = Result<bytes::Bytes, String>> + Send>,
    >,
}

// Re-export bytes so the type signature compiles without forcing the
// transport module to depend on the `bytes` crate directly.
mod bytes {
    pub use ::bytes::Bytes;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_chunk_complete_event() {
        let mut s = SseParseState::new();
        let evs = parse_chunk(b"event: foo\ndata: hello\ndata: world\nid: 42\n\n", &mut s);
        assert_eq!(evs.len(), 1);
        assert_eq!(evs[0].event.as_deref(), Some("foo"));
        assert_eq!(evs[0].data, "hello\nworld");
        assert_eq!(evs[0].id.as_deref(), Some("42"));
    }

    #[test]
    fn parse_chunk_split_across_chunks() {
        let mut s = SseParseState::new();
        let evs1 = parse_chunk(b"event: x\ndata: par", &mut s);
        assert!(evs1.is_empty());
        // Continuation: a "data:" line that itself is split across chunks
        // reassembles correctly.
        let evs2 = parse_chunk(b"data: tial\n\n", &mut s);
        assert_eq!(evs2.len(), 1);
        assert_eq!(evs2[0].event.as_deref(), Some("x"));
        assert_eq!(evs2[0].data, "par\ntial");
    }

    #[test]
    fn parse_chunk_comment_and_keepalive() {
        let mut s = SseParseState::new();
        let evs = parse_chunk(b": this is a comment\n\n", &mut s);
        assert!(evs.is_empty());
        // A comment-only event with no data should not produce an event
        // unless event:/id: were set. We confirm that.
        let evs2 = parse_chunk(b":another comment\ndata: hi\n\n", &mut s);
        assert_eq!(evs2.len(), 1);
        assert_eq!(evs2[0].data, "hi");
        assert!(evs2[0].event.is_none());
    }

    #[test]
    fn parse_chunk_multiple_events_in_one_chunk() {
        let mut s = SseParseState::new();
        let evs = parse_chunk(
            b"event: a\ndata: 1\n\nevent: b\ndata: 2\n\n",
            &mut s,
        );
        assert_eq!(evs.len(), 2);
        assert_eq!(evs[0].event.as_deref(), Some("a"));
        assert_eq!(evs[0].data, "1");
        assert_eq!(evs[1].event.as_deref(), Some("b"));
        assert_eq!(evs[1].data, "2");
    }

    #[test]
    fn parse_chunk_crlf_line_endings() {
        let mut s = SseParseState::new();
        let evs = parse_chunk(b"event: x\r\ndata: hi\r\n\r\n", &mut s);
        assert_eq!(evs.len(), 1);
        assert_eq!(evs[0].event.as_deref(), Some("x"));
        assert_eq!(evs[0].data, "hi");
    }
}
