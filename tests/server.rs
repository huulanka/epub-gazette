mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use http_body_util::BodyExt;
use tower::ServiceExt;

use epub_gazette::server::{Config, router};

async fn send(app: axum::Router, req: Request<Body>) -> (StatusCode, Option<String>, Vec<u8>) {
    let resp = app.oneshot(req).await.unwrap();
    let status = resp.status();
    let ct = resp
        .headers()
        .get(header::CONTENT_TYPE)
        .map(|v| v.to_str().unwrap().to_string());
    let body = resp
        .into_body()
        .collect()
        .await
        .unwrap()
        .to_bytes()
        .to_vec();
    (status, ct, body)
}

fn post(path: &str, body: String) -> Request<Body> {
    Request::post(path)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(body))
        .unwrap()
}

#[tokio::test]
async fn healthz() {
    let (status, _, body) = send(
        router(Config::default()),
        Request::get("/healthz").body(Body::empty()).unwrap(),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body, b"ok");
}

#[tokio::test]
async fn render_returns_an_epub() {
    let (status, ct, body) = send(
        router(Config::default()),
        post("/v1/render", common::sample_json()),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(ct.as_deref(), Some("application/epub+zip"));
    assert_eq!(&body[30..50], b"mimetypeapplication/");
}

#[tokio::test]
async fn cover_returns_a_jpeg() {
    let (status, ct, body) = send(
        router(Config::default()),
        post("/v1/cover", common::sample_json()),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(ct.as_deref(), Some("image/jpeg"));
    assert_eq!(&body[..2], &[0xFF, 0xD8]);
}

#[tokio::test]
async fn malformed_json_is_422_with_json_error() {
    let (status, ct, body) = send(
        router(Config::default()),
        post("/v1/render", "{\"title\": 1}".into()),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(ct.as_deref(), Some("application/json"));
    let v: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert!(v["error"].as_str().unwrap().contains("title"));
}

#[tokio::test]
async fn semantic_errors_are_listed() {
    let mut issue: serde_json::Value = serde_json::from_str(&common::sample_json()).unwrap();
    issue["sections"][0]["articles"] = serde_json::json!([]);
    let (status, _, body) = send(
        router(Config::default()),
        post("/v1/render", issue.to_string()),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    let v: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(v["details"][0], "sections[0] has no articles");
}

#[tokio::test]
async fn token_is_enforced_when_configured() {
    let app = router(Config {
        token: Some("s3cret".into()),
    });
    let (status, _, _) = send(
        app.clone(),
        Request::get("/v1/themes").body(Body::empty()).unwrap(),
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    let req = Request::get("/v1/themes")
        .header(header::AUTHORIZATION, "Bearer s3cret")
        .body(Body::empty())
        .unwrap();
    let (status, _, body) = send(app.clone(), req).await;
    assert_eq!(status, StatusCode::OK);
    assert!(String::from_utf8(body).unwrap().contains("klassisch-1900"));
    // health checks stay open for Docker
    let (status, _, _) = send(app, Request::get("/healthz").body(Body::empty()).unwrap()).await;
    assert_eq!(status, StatusCode::OK);
}

#[tokio::test]
async fn schema_endpoint_serves_json_schema() {
    let (status, _, body) = send(
        router(Config::default()),
        Request::get("/v1/schema").body(Body::empty()).unwrap(),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let v: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(v["title"], "epub-gazette issue (v1)");
    assert!(v["properties"]["sections"].is_object());
}
