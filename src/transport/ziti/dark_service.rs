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
use ziti_sdk::ZitiStream;

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
        let mut inner = self.inner.clone();
        Box::pin(async move {
            let resp = inner
                .oneshot(req)
                .await
                .expect("axum::Router oneshot is infallible");
            Ok::<_, Infallible>(resp)
        })
    }
}

/// Convenience: serve one axum `Router` over one accepted
/// `ZitiStream` until the peer disconnects.
pub async fn serve_ziti_stream(stream: ZitiStream, app: axum::Router) {
    let service = AxumService { inner: app };
    let io = TokioIo::new(stream);
    if let Err(e) = Builder::new()
        .preserve_header_case(true)
        .serve_connection(io, service)
        .await
    {
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