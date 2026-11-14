//! HTTP API.
//!
//! | Method | Path         | Body          | Response                    |
//! |--------|--------------|---------------|-----------------------------|
//! | POST   | `/v1/render` | `issue.v1`    | `application/epub+zip`      |
//! | POST   | `/v1/cover`  | `issue.v1`    | `image/jpeg` (preview)      |
//! | GET    | `/v1/schema` |               | JSON Schema of `issue.v1`   |
//! | GET    | `/v1/themes` |               | available themes            |
//! | GET    | `/healthz`   |               | `ok`                        |
//!
//! Errors are JSON: `{"error": "...", "details": [...]}`.

use axum::extract::rejection::JsonRejection;
use axum::extract::{DefaultBodyLimit, Request, State};
use axum::http::{HeaderValue, StatusCode, header};
use axum::middleware::{self, Next};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde_json::json;
use tower_http::trace::TraceLayer;

use crate::model::Issue;
use crate::render::{self, RenderError};

/// Requests may carry a base64 image of up to 12 MiB.
pub const BODY_LIMIT: usize = 20 * 1024 * 1024;

#[derive(Clone, Default)]
pub struct Config {
    /// When set, every `/v1/*` request needs `Authorization: Bearer <token>`.
    pub token: Option<String>,
}

pub fn router(config: Config) -> Router {
    let api = Router::new()
        .route("/v1/render", post(render_epub))
        .route("/v1/cover", post(render_cover))
        .route("/v1/schema", get(schema))
        .route("/v1/themes", get(themes))
        .route_layer(middleware::from_fn_with_state(config.clone(), auth))
        .with_state(config);
    Router::new()
        .route("/healthz", get(|| async { "ok" }))
        .merge(api)
        .layer(DefaultBodyLimit::max(BODY_LIMIT))
        .layer(TraceLayer::new_for_http())
}

pub struct ApiError {
    status: StatusCode,
    message: String,
    details: Vec<String>,
}

impl ApiError {
    fn new(status: StatusCode, message: impl Into<String>) -> Self {
        Self {
            status,
            message: message.into(),
            details: vec![],
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (
            self.status,
            Json(json!({ "error": self.message, "details": self.details })),
        )
            .into_response()
    }
}

impl From<JsonRejection> for ApiError {
    fn from(r: JsonRejection) -> Self {
        ApiError::new(StatusCode::UNPROCESSABLE_ENTITY, r.body_text())
    }
}

impl From<RenderError> for ApiError {
    fn from(e: RenderError) -> Self {
        match e {
            RenderError::Invalid(details) => ApiError {
                status: StatusCode::UNPROCESSABLE_ENTITY,
                message: "the issue is invalid".into(),
                details,
            },
            RenderError::Graphics(g) => {
                ApiError::new(StatusCode::UNPROCESSABLE_ENTITY, g.to_string())
            }
            other => {
                tracing::error!(error = %other, "render failed");
                ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, other.to_string())
            }
        }
    }
}

async fn auth(State(cfg): State<Config>, req: Request, next: Next) -> Result<Response, ApiError> {
    if let Some(token) = &cfg.token {
        let ok = req
            .headers()
            .get(header::AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.strip_prefix("Bearer "))
            .is_some_and(|t| constant_time_eq(t.as_bytes(), token.as_bytes()));
        if !ok {
            return Err(ApiError::new(
                StatusCode::UNAUTHORIZED,
                "missing or wrong bearer token",
            ));
        }
    }
    Ok(next.run(req).await)
}

fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    a.len() == b.len() && a.iter().zip(b).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
}

/// Rendering is CPU-bound (image resampling, SVG rasterization, zip), so it
/// runs on the blocking pool instead of stalling the async workers.
async fn blocking<T: Send + 'static>(
    f: impl FnOnce() -> Result<T, RenderError> + Send + 'static,
) -> Result<T, ApiError> {
    tokio::task::spawn_blocking(f)
        .await
        .map_err(|e| {
            ApiError::new(
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("render task failed: {e}"),
            )
        })?
        .map_err(ApiError::from)
}

async fn render_epub(payload: Result<Json<Issue>, JsonRejection>) -> Result<Response, ApiError> {
    let Json(issue) = payload?;
    let rendered = blocking(move || render::render(issue)).await?;
    let disposition = format!("attachment; filename=\"{}\"", rendered.file_name);
    Ok((
        [
            (
                header::CONTENT_TYPE,
                HeaderValue::from_static("application/epub+zip"),
            ),
            (
                header::CONTENT_DISPOSITION,
                HeaderValue::from_str(&disposition)
                    .unwrap_or(HeaderValue::from_static("attachment")),
            ),
        ],
        rendered.epub,
    )
        .into_response())
}

async fn render_cover(payload: Result<Json<Issue>, JsonRejection>) -> Result<Response, ApiError> {
    let Json(issue) = payload?;
    let jpg = blocking(move || render::render_cover(issue)).await?;
    Ok((
        [(header::CONTENT_TYPE, HeaderValue::from_static("image/jpeg"))],
        jpg,
    )
        .into_response())
}

async fn schema() -> Json<serde_json::Value> {
    Json(crate::schema())
}

async fn themes() -> Json<serde_json::Value> {
    Json(json!(
        crate::theme::all()
            .iter()
            .map(|t| json!({ "id": t.id, "name": t.name, "description": t.description }))
            .collect::<Vec<_>>()
    ))
}
