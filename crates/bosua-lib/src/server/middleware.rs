//! API key authentication middleware.
//!
//! Reads `BOSUA_API_KEY` from the environment. When set, every request to a
//! protected endpoint must include a matching `X-API-Key` header. If no key
//! is configured the middleware passes all requests through (with a warning
//! in verbose mode).

use axum::{
    extract::Request,
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
};

/// Header name used to carry the API key.
pub const API_KEY_HEADER: &str = "X-API-Key";

/// Environment variable that holds the expected API key.
pub const API_KEY_ENV: &str = "BOSUA_API_KEY";

/// Axum middleware that enforces API key authentication.
///
/// If `BOSUA_API_KEY` is set, the request must include a matching
/// `X-API-Key` header. Otherwise the request is rejected with 401.
///
/// If the env var is not set, all requests are allowed through and a
/// warning is logged once (at debug level, visible in verbose mode).
pub async fn auth_middleware(req: Request, next: Next) -> Response {
    let expected_key = std::env::var(API_KEY_ENV).ok();
    validate_api_key(expected_key.as_deref(), req, next).await
}

/// Core validation logic, separated for testability.
///
/// `expected_key` is the configured API key (or `None` if not configured).
async fn validate_api_key(
    expected_key: Option<&str>,
    req: Request,
    next: Next,
) -> Response {
    match expected_key {
        Some(expected) if !expected.is_empty() => {
            let provided = req
                .headers()
                .get(API_KEY_HEADER)
                .and_then(|v| v.to_str().ok());

            match provided {
                Some(key) if key == expected => next.run(req).await,
                _ => {
                    tracing::warn!("Unauthorized request: invalid or missing API key");
                    (
                        StatusCode::UNAUTHORIZED,
                        axum::Json(serde_json::json!({
                            "success": false,
                            "message": "Unauthorized: invalid or missing API key"
                        })),
                    )
                        .into_response()
                }
            }
        }
        _ => {
            tracing::debug!("No BOSUA_API_KEY configured — authentication disabled");
            next.run(req).await
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::Body,
        http::{Request as HttpRequest, StatusCode},
        middleware,
        routing::get,
        Router,
    };
    use tower::ServiceExt;

    async fn ok_handler() -> &'static str {
        "ok"
    }

    /// Build a router that uses a middleware with a fixed expected key.
    fn router_with_key(key: Option<&'static str>) -> Router {
        Router::new()
            .route("/protected", get(ok_handler))
            .layer(middleware::from_fn(move |req, next| {
                validate_api_key(key, req, next)
            }))
    }

    #[tokio::test]
    async fn allows_request_when_no_key_configured() {
        let app = router_with_key(None);
        let req = HttpRequest::builder()
            .uri("/protected")
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn rejects_request_without_key_when_configured() {
        let app = router_with_key(Some("test-secret-key"));
        let req = HttpRequest::builder()
            .uri("/protected")
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn allows_request_with_valid_key() {
        let app = router_with_key(Some("test-secret-key"));
        let req = HttpRequest::builder()
            .uri("/protected")
            .header(API_KEY_HEADER, "test-secret-key")
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn rejects_request_with_wrong_key() {
        let app = router_with_key(Some("test-secret-key"));
        let req = HttpRequest::builder()
            .uri("/protected")
            .header(API_KEY_HEADER, "wrong-key")
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn allows_when_key_is_empty_string() {
        let app = router_with_key(Some(""));
        let req = HttpRequest::builder()
            .uri("/protected")
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }
}
