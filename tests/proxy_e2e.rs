use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::response::IntoResponse;
use axum::routing::post;
use serde_json::{json, Value};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::sync::Mutex;
use zed_openai_shim::AppState;

#[derive(Default)]
struct Capture {
    payloads: Mutex<Vec<Value>>,
    authorizations: Mutex<Vec<Option<String>>>,
    paths: Mutex<Vec<String>>,
}

#[tokio::test]
async fn proxy_moves_system_to_instructions_and_overrides_authorization() {
    let capture = Arc::new(Capture::default());
    let upstream_address = spawn_upstream(capture.clone()).await;
    let shim_address = spawn_shim(upstream_address).await;

    let client = reqwest::Client::new();
    let response = client
        .post(format!("http://{shim_address}/v1/responses"))
        .header("content-type", "application/json")
        .header("authorization", "Bearer dummy")
        .json(&json!({
            "model": "gpt-5.5",
            "input": [
                {
                    "type": "message",
                    "role": "system",
                    "content": [{"type": "input_text", "text": "You are a terse test assistant."}]
                },
                {
                    "type": "message",
                    "role": "user",
                    "content": [{"type": "input_text", "text": "Reply with ok."}]
                }
            ],
            "stream": true
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.json::<Value>().await.unwrap(), json!({"ok": true}));

    let payloads = capture.payloads.lock().await;
    assert_eq!(payloads.len(), 1);
    assert_eq!(
        payloads[0]["instructions"],
        "You are a terse test assistant."
    );
    assert_eq!(payloads[0]["input"].as_array().unwrap().len(), 1);
    assert_eq!(payloads[0]["input"][0]["role"], "user");

    let authorizations = capture.authorizations.lock().await;
    assert_eq!(
        authorizations.as_slice(),
        &[Some("Bearer test-upstream-key".to_string())]
    );

    let paths = capture.paths.lock().await;
    assert_eq!(paths.as_slice(), &["/v1/responses".to_string()]);
}

#[tokio::test]
async fn proxy_preserves_query_string() {
    let capture = Arc::new(Capture::default());
    let upstream_address = spawn_upstream(capture.clone()).await;
    let shim_address = spawn_shim(upstream_address).await;

    let response = reqwest::Client::new()
        .post(format!("http://{shim_address}/v1/responses?trace=1"))
        .json(&json!({"input": []}))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let paths = capture.paths.lock().await;
    assert_eq!(paths.as_slice(), &["/v1/responses?trace=1".to_string()]);
}

async fn spawn_upstream(capture: Arc<Capture>) -> SocketAddr {
    async fn handler(
        State(capture): State<Arc<Capture>>,
        headers: HeaderMap,
        uri: axum::http::Uri,
        body: String,
    ) -> impl IntoResponse {
        let payload = serde_json::from_str::<Value>(&body).unwrap();
        capture.payloads.lock().await.push(payload);
        capture.paths.lock().await.push(uri.to_string());
        capture.authorizations.lock().await.push(
            headers
                .get("authorization")
                .and_then(|value| value.to_str().ok())
                .map(str::to_owned),
        );
        (StatusCode::OK, axum::Json(json!({"ok": true})))
    }

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let app = axum::Router::new()
        .route("/v1/responses", post(handler))
        .with_state(capture);
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    address
}

async fn spawn_shim(upstream_address: SocketAddr) -> SocketAddr {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let state = AppState::new(
        format!("http://{upstream_address}"),
        Some("test-upstream-key".to_string()),
        true,
        reqwest::Client::new(),
    )
    .unwrap();
    tokio::spawn(async move {
        state.serve(listener, std::future::pending()).await.unwrap();
    });
    address
}
