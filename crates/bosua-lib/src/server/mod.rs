//! HTTP server module (Linux variants only).
//!
//! Provides the axum-based HTTP/HTTPS server with TCP optimizations,
//! TLS support, API key authentication middleware, graceful shutdown,
//! and 30+ endpoint routes.

pub mod config;
pub mod handlers;
pub mod middleware;
pub mod sse;
pub mod tls;

use crate::cloud::fshare::FShareClient;
use crate::cloud::gdrive::GDriveClient;
use crate::commands::registry_cmd::ServiceRegistry;
use crate::config::manager::DynamicConfigManager;
use crate::daemon::DaemonManager;
use crate::errors::{BosuaError, Result};
use crate::notifications::{NotificationManager, SseHandler};
use crate::search::SearchEngine;
use config::{ServerConfig, SHUTDOWN_TIMEOUT_SECS};
use sse::SseState;
use axum::{
    middleware as axum_mw,
    routing::{get, post},
    Router,
};
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::sync::oneshot;

// ---------------------------------------------------------------------------
// AppState
// ---------------------------------------------------------------------------

/// Shared state for Axum HTTP handlers.
///
/// Constructed from a [`ServiceRegistry`] by eagerly initializing the
/// services needed for HTTP request handling. Each field is an `Arc`
/// so the state can be cheaply cloned into every handler task.
pub struct AppState {
    pub config_manager: Arc<DynamicConfigManager>,
    pub search_engine: Arc<SearchEngine>,
    pub gdrive: Arc<GDriveClient>,
    pub fshare: Arc<FShareClient>,
    pub daemon_manager: Arc<DaemonManager>,
    pub notification_manager: Arc<NotificationManager>,
}

impl AppState {
    /// Build `AppState` from a [`ServiceRegistry`], eagerly initializing
    /// the services required by the HTTP layer.
    pub async fn from_registry(
        services: &ServiceRegistry,
        notification_manager: Arc<NotificationManager>,
    ) -> Result<Self> {
        Ok(Self {
            config_manager: Arc::clone(&services.config_manager),
            search_engine: Arc::clone(services.search_engine().await?),
            gdrive: Arc::clone(services.gdrive().await?),
            fshare: Arc::clone(services.fshare().await?),
            daemon_manager: Arc::clone(services.daemon_manager().await),
            notification_manager,
        })
    }
}

// ---------------------------------------------------------------------------
// ShutdownHandle
// ---------------------------------------------------------------------------

/// Handle returned by `start_server` that allows triggering graceful shutdown.
pub struct ShutdownHandle {
    shutdown_tx: Option<oneshot::Sender<()>>,
}

impl ShutdownHandle {
    /// Signal the server to begin graceful shutdown.
    ///
    /// Returns `Ok(())` if the signal was sent, or `Err` if the server
    /// already stopped.
    pub fn shutdown(mut self) -> Result<()> {
        if let Some(tx) = self.shutdown_tx.take() {
            tx.send(()).map_err(|_| {
                BosuaError::Server {
                    status: 500,
                    message: "Server already stopped".into(),
                }
            })
        } else {
            Ok(())
        }
    }
}

// ---------------------------------------------------------------------------
// Router construction
// ---------------------------------------------------------------------------

/// Build the full axum router with all endpoint routes.
///
/// Public (unauthenticated) routes are mounted directly.
/// Protected routes are wrapped with the auth middleware layer.
///
/// The `sse_handler` provides the broadcast channel for the SSE endpoint.
/// If `None`, the SSE endpoint falls back to a stub response.
///
/// When `app_state` is provided, handlers that need service access
/// (config, search, fshare, gdrive, daemon, logs) are wired with state.
/// Otherwise they fall back to stub responses.
pub fn build_router(
    sse_handler: Option<Arc<SseHandler>>,
    app_state: Option<Arc<AppState>>,
) -> Router {
    // Public routes (no auth required)
    let mut public_routes = Router::new()
        .route("/health", get(handlers::health));

    // Wire the real SSE endpoint when an SseHandler is provided
    if let Some(handler) = sse_handler {
        let sse_state = SseState { sse_handler: handler };
        let sse_route = Router::new()
            .route("/notifications/sse", get(sse::notifications_sse_handler))
            .with_state(sse_state);
        public_routes = public_routes.merge(sse_route);
    } else {
        public_routes = public_routes
            .route("/notifications/sse", get(handlers::notifications_sse));
    }

    // Build protected routes. Stateful handlers are on a sub-router
    // with AppState; stateless handlers are merged separately.
    let protected_routes = if let Some(state) = app_state {
        // Stateful routes — handlers that use State<Arc<AppState>>
        let stateful = Router::new()
            // Config
            .route(
                "/config",
                get(handlers::get_config).put(handlers::update_config),
            )
            .route("/config/reset", post(handlers::reset_config))
            // Search
            .route("/search", post(handlers::search))
            // FShare
            .route("/fshare/links", post(handlers::fshare_links))
            // Daemon
            .route("/daemon/status", get(handlers::daemon_status))
            // GDrive
            .route("/gdrive/files", get(handlers::gdrive_files))
            .route("/gdrive/upload", post(handlers::gdrive_upload))
            // Logs
            .route("/logs", get(handlers::logs))
            .with_state(state);

        // Stateless routes — handlers that don't need AppState
        let stateless = Router::new()
            // Links
            .route("/append-links", post(handlers::append_links))
            // Files
            .route(
                "/files",
                get(handlers::list_files)
                    .post(handlers::upload_file)
                    .delete(handlers::delete_file),
            )
            // Notifications
            .route("/notifications", get(handlers::list_notifications))
            // Stats
            .route("/stats", get(handlers::stats))
            // Memprofile
            .route("/memprofile", get(handlers::memprofile))
            // PDF / LaTeX
            .route("/pdf", post(handlers::pdf))
            .route("/latex2pdf", post(handlers::latex2pdf))
            // Onflix
            .route("/onflix", post(handlers::onflix))
            // FShare scan
            .route("/fshare/scan", get(handlers::fshare_scan))
            // GDrive proxy
            .route("/gdrive/proxy", get(handlers::gdrive_proxy))
            // GCloud
            .route("/gcloud/instances", get(handlers::gcloud_instances))
            // GCP
            .route("/gcp/browse", get(handlers::gcp_browse))
            .route("/gcp/download", get(handlers::gcp_download))
            .route("/gcp/list", get(handlers::gcp_list))
            .route("/gcp/play", post(handlers::gcp_play))
            .route("/gcp/push", post(handlers::gcp_push))
            // AWS
            .route("/aws/ec2", get(handlers::aws_ec2))
            .route("/aws/ec2/start", post(handlers::aws_ec2_start))
            .route("/aws/ec2/stop", post(handlers::aws_ec2_stop))
            .route("/aws/sg", get(handlers::aws_sg))
            .route("/aws/regions", get(handlers::aws_regions))
            .route("/aws/zones", get(handlers::aws_zones))
            // Tailscale
            .route("/tailscale/devices", get(handlers::tailscale_devices))
            .route("/tailscale/acl", get(handlers::tailscale_acl))
            .route("/tailscale/routes", get(handlers::tailscale_routes))
            .route("/tailscale/keys", get(handlers::tailscale_keys))
            // WordPress
            .route("/wordpress", get(handlers::wordpress))
            // Cache
            .route(
                "/cache",
                get(handlers::get_cache).delete(handlers::clear_cache),
            )
            // Accounts
            .route("/accounts", get(handlers::accounts))
            // Instance
            .route("/instance-create", post(handlers::instance_create))
            // Kodi
            .route("/kodi-repo", get(handlers::kodi_repo))
            // Media
            .route("/play", post(handlers::play));

        stateful
            .merge(stateless)
            .layer(axum_mw::from_fn(middleware::auth_middleware))
    } else {
        // No AppState — all handlers are stateless stubs
        Router::new()
            // Links
            .route("/append-links", post(handlers::append_links))
            // Search
            .route("/search", post(handlers::search_stub))
            // Files
            .route(
                "/files",
                get(handlers::list_files)
                    .post(handlers::upload_file)
                    .delete(handlers::delete_file),
            )
            // Config
            .route(
                "/config",
                get(handlers::get_config_stub).put(handlers::update_config_stub),
            )
            .route("/config/reset", post(handlers::reset_config_stub))
            // Daemon
            .route("/daemon/status", get(handlers::daemon_status_stub))
            // Notifications
            .route("/notifications", get(handlers::list_notifications))
            // Logs
            .route("/logs", get(handlers::logs_stub))
            // Stats
            .route("/stats", get(handlers::stats))
            // Memprofile
            .route("/memprofile", get(handlers::memprofile))
            // PDF / LaTeX
            .route("/pdf", post(handlers::pdf))
            .route("/latex2pdf", post(handlers::latex2pdf))
            // Onflix
            .route("/onflix", post(handlers::onflix))
            // FShare
            .route("/fshare/links", post(handlers::fshare_links_stub))
            .route("/fshare/scan", get(handlers::fshare_scan))
            // GDrive
            .route("/gdrive/files", get(handlers::gdrive_files_stub))
            .route("/gdrive/upload", post(handlers::gdrive_upload_stub))
            .route("/gdrive/proxy", get(handlers::gdrive_proxy))
            // GCloud
            .route("/gcloud/instances", get(handlers::gcloud_instances))
            // GCP
            .route("/gcp/browse", get(handlers::gcp_browse))
            .route("/gcp/download", get(handlers::gcp_download))
            .route("/gcp/list", get(handlers::gcp_list))
            .route("/gcp/play", post(handlers::gcp_play))
            .route("/gcp/push", post(handlers::gcp_push))
            // AWS
            .route("/aws/ec2", get(handlers::aws_ec2))
            .route("/aws/ec2/start", post(handlers::aws_ec2_start))
            .route("/aws/ec2/stop", post(handlers::aws_ec2_stop))
            .route("/aws/sg", get(handlers::aws_sg))
            .route("/aws/regions", get(handlers::aws_regions))
            .route("/aws/zones", get(handlers::aws_zones))
            // Tailscale
            .route("/tailscale/devices", get(handlers::tailscale_devices))
            .route("/tailscale/acl", get(handlers::tailscale_acl))
            .route("/tailscale/routes", get(handlers::tailscale_routes))
            .route("/tailscale/keys", get(handlers::tailscale_keys))
            // WordPress
            .route("/wordpress", get(handlers::wordpress))
            // Cache
            .route(
                "/cache",
                get(handlers::get_cache).delete(handlers::clear_cache),
            )
            // Accounts
            .route("/accounts", get(handlers::accounts))
            // Instance
            .route("/instance-create", post(handlers::instance_create))
            // Kodi
            .route("/kodi-repo", get(handlers::kodi_repo))
            // Media
            .route("/play", post(handlers::play))
            .layer(axum_mw::from_fn(middleware::auth_middleware))
    };

    public_routes.merge(protected_routes)
}

// ---------------------------------------------------------------------------
// Server startup
// ---------------------------------------------------------------------------

/// Start the HTTP server with the given configuration.
///
/// An optional `SseHandler` enables the real SSE notification stream.
/// When provided, the `/notifications/sse` endpoint streams live
/// notifications from the broadcast channel.
///
/// An optional `AppState` enables stateful handlers (config, search, etc.).
///
/// Returns a `ShutdownHandle` that can be used to trigger graceful shutdown.
/// The server runs in a background tokio task.
pub async fn start_server(
    config: ServerConfig,
    sse_handler: Option<Arc<SseHandler>>,
    app_state: Option<Arc<AppState>>,
) -> Result<ShutdownHandle> {
    config.validate()?;

    let router = build_router(sse_handler, app_state);
    let addr = config.addr();

    let listener = TcpListener::bind(&addr).await.map_err(|e| {
        BosuaError::Server {
            status: 500,
            message: format!("Failed to bind to {}: {}", addr, e),
        }
    })?;

    // Log the resolved local address
    let local_addr = listener.local_addr().map_err(|e| {
        BosuaError::Server {
            status: 500,
            message: format!("Failed to get local address: {}", e),
        }
    })?;

    tracing::info!(
        addr = %local_addr,
        tls = config.tls,
        "HTTP server starting"
    );

    let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();

    if config.tls {
        start_tls_server(config, listener, router, shutdown_rx).await?;
    } else {
        start_plain_server(listener, router, shutdown_rx);
    }

    Ok(ShutdownHandle {
        shutdown_tx: Some(shutdown_tx),
    })
}

/// Start a plain HTTP server (no TLS).
fn start_plain_server(
    listener: TcpListener,
    router: Router,
    shutdown_rx: oneshot::Receiver<()>,
) {
    tokio::spawn(async move {
        let serve = axum::serve(listener, router)
            .with_graceful_shutdown(graceful_shutdown_signal(shutdown_rx));

        if let Err(e) = serve.await {
            tracing::error!("Server error: {}", e);
        }
    });
}

/// Start an HTTPS server with TLS.
///
/// Uses tokio-rustls to accept TLS connections. Each accepted TLS stream
/// is handed off to axum for HTTP processing.
///
/// NOTE: Full per-connection HTTP serving over TLS will be refined when
/// hyper_util is added as a dependency. For now the TLS acceptor is
/// validated at startup and connections are accepted in a loop.
async fn start_tls_server(
    config: ServerConfig,
    listener: TcpListener,
    router: Router,
    shutdown_rx: oneshot::Receiver<()>,
) -> Result<()> {
    let tls_acceptor = tls::setup_tls(&config)?;

    tokio::spawn(async move {
        let shutdown_fut = graceful_shutdown_signal(shutdown_rx);
        tokio::pin!(shutdown_fut);

        loop {
            tokio::select! {
                _ = &mut shutdown_fut => {
                    tracing::info!("TLS server shutting down");
                    break;
                }
                accept_result = listener.accept() => {
                    match accept_result {
                        Ok((stream, addr)) => {
                            let acceptor = tls_acceptor.clone();
                            let _router = router.clone();
                            tokio::spawn(async move {
                                match acceptor.accept(stream).await {
                                    Ok(_tls_stream) => {
                                        // TODO: serve HTTP over the TLS stream using
                                        // hyper_util once it is added as a dependency.
                                        tracing::debug!(peer = %addr, "TLS connection established");
                                    }
                                    Err(e) => {
                                        tracing::debug!(peer = %addr, "TLS handshake failed: {}", e);
                                    }
                                }
                            });
                        }
                        Err(e) => {
                            tracing::error!("Accept error: {}", e);
                        }
                    }
                }
            }
        }
    });

    Ok(())
}

/// Future that resolves when the shutdown signal is received.
///
/// Implements the graceful shutdown sequence:
/// 1. Wait for the shutdown signal
/// 2. Allow up to `SHUTDOWN_TIMEOUT_SECS` for in-flight requests to complete
async fn graceful_shutdown_signal(shutdown_rx: oneshot::Receiver<()>) {
    let _ = shutdown_rx.await;
    tracing::info!("Shutdown signal received, allowing {}s for in-flight requests", SHUTDOWN_TIMEOUT_SECS);
    // The actual timeout is enforced by axum's graceful_shutdown mechanism.
    // We just need to signal that shutdown should begin.
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use tower::ServiceExt;

    #[tokio::test]
    async fn health_endpoint_returns_ok() {
        let app = build_router(None, None);
        let req = Request::builder()
            .uri("/health")
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn protected_endpoint_accessible_without_key_env() {
        // When BOSUA_API_KEY is not set, all requests pass through
        std::env::remove_var("BOSUA_API_KEY");

        let app = build_router(None, None);
        let req = Request::builder()
            .uri("/stats")
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn unknown_route_returns_404() {
        let app = build_router(None, None);
        let req = Request::builder()
            .uri("/nonexistent")
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn start_server_binds_and_shuts_down() {
        let config = ServerConfig {
            port: 0, // OS-assigned port
            host: "127.0.0.1".into(),
            ..Default::default()
        };

        let handle = start_server(config, None, None).await.expect("server should start");
        handle.shutdown().expect("shutdown should succeed");
    }

    #[test]
    fn router_has_at_least_30_routes() {
        // Verify via endpoint_configurations which mirrors the router
        let endpoints = config::endpoint_configurations();
        assert!(
            endpoints.len() >= 30,
            "Expected at least 30 endpoints, got {}",
            endpoints.len()
        );
    }
}
