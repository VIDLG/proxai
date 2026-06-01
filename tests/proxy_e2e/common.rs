use axum::body::{Body, Bytes};
use axum::extract::State;
use axum::http::{HeaderMap, Response, StatusCode};
use axum::response::IntoResponse;
use axum::routing::post;
use futures_util::{stream, StreamExt};
use proxai::AppState;
use serde_json::{json, Value};
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::fs;
use tokio::net::TcpListener;
use tokio::sync::Mutex;

#[derive(Default)]
pub(super) struct Capture {
    pub(super) payloads: Mutex<Vec<Value>>,
    pub(super) authorizations: Mutex<Vec<Option<String>>>,
    pub(super) api_keys: Mutex<Vec<Option<String>>>,
    pub(super) paths: Mutex<Vec<String>>,
}

static CAPTURE_DIR_COUNTER: AtomicU64 = AtomicU64::new(0);

pub(super) fn responses_request(stream: bool) -> Value {
    json!({
        "model": "gpt-5.5",
        "input": [{
            "type": "message",
            "role": "user",
            "content": [{"type": "input_text", "text": "hello"}]
        }],
        "stream": stream
    })
}

pub(super) fn local_client() -> reqwest::Client {
    reqwest::Client::builder()
        .no_proxy()
        .build()
        .expect("build local proxy test client")
}

pub(super) async fn spawn_upstream(capture: Arc<Capture>) -> SocketAddr {
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
        capture.api_keys.lock().await.push(
            headers
                .get("x-api-key")
                .and_then(|value| value.to_str().ok())
                .map(str::to_owned),
        );
        (StatusCode::OK, axum::Json(json!({"ok": true})))
    }

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let app = axum::Router::new()
        .route("/v1/responses", post(handler))
        .route("/v1/chat/completions", post(handler))
        .route("/v1/messages", post(handler))
        .with_state(capture);
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    address
}

pub(super) async fn spawn_anthropic_messages_compat_upstream(capture: Arc<Capture>) -> SocketAddr {
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
        capture.api_keys.lock().await.push(
            headers
                .get("x-api-key")
                .and_then(|value| value.to_str().ok())
                .map(str::to_owned),
        );
        (
            StatusCode::OK,
            axum::Json(json!({
                "id": "msg_compat",
                "role": "assistant",
                "model": "glm-5.1",
                "content": [
                    {"type": "tool_use", "id": "toolu_1", "name": "lookup", "input": {}}
                ],
                "stop_reason": "tool_use",
                "stop_sequence": null,
                "usage": {
                    "input_tokens": 10,
                    "output_tokens": 4,
                    "server_tool_use": {"web_search_requests": 1}
                }
            })),
        )
    }

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let app = axum::Router::new()
        .route("/v1/messages", post(handler))
        .with_state(capture);
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    address
}

pub(super) async fn spawn_error_upstream(capture: Arc<Capture>) -> SocketAddr {
    async fn handler(
        State(capture): State<Arc<Capture>>,
        headers: HeaderMap,
        uri: axum::http::Uri,
        body: String,
    ) -> Response<Body> {
        let payload = serde_json::from_str::<Value>(&body).unwrap();
        capture.payloads.lock().await.push(payload);
        capture.paths.lock().await.push(uri.to_string());
        capture.authorizations.lock().await.push(
            headers
                .get("authorization")
                .and_then(|value| value.to_str().ok())
                .map(str::to_owned),
        );
        capture.api_keys.lock().await.push(
            headers
                .get("x-api-key")
                .and_then(|value| value.to_str().ok())
                .map(str::to_owned),
        );

        Response::builder()
            .status(StatusCode::TOO_MANY_REQUESTS)
            .header("content-type", "application/json")
            .header("retry-after", "7")
            .header("x-request-id", "req_test_123")
            .header("request-id", "anthropic_req_123")
            .header("x-ratelimit-remaining-requests", "0")
            .header("anthropic-ratelimit-requests-remaining", "0")
            .body(Body::from(
                r#"{"error":{"message":"quota exhausted","code":"rate_limit_exceeded"}}"#,
            ))
            .unwrap()
    }

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let app = axum::Router::new()
        .route("/v1/responses", post(handler))
        .route("/v1/chat/completions", post(handler))
        .route("/v1/messages", post(handler))
        .with_state(capture);
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    address
}

pub(super) async fn spawn_chat_completion_sse_upstream() -> SocketAddr {
    async fn stream() -> Response<Body> {
        let chunks = stream::iter([
            Ok::<_, std::io::Error>(Bytes::from_static(
                b"data: {\"id\":\"chatcmpl_stream\",\"object\":\"chat.completion.chunk\",\"created\":1,\"model\":\"gpt-4.1\",\"choices\":[{\"index\":0,\"delta\":{\"role\":\"assistant\",\"content\":\"ok\"},\"finish_reason\":\"stop\"}],\"usage\":{\"prompt_tokens\":8,\"completion_tokens\":2,\"total_tokens\":10}}\n\n",
            )),
            Ok(Bytes::from_static(b"data: [DONE]\n\n")),
        ]);

        Response::builder()
            .status(StatusCode::OK)
            .header("content-type", "text/event-stream")
            .body(Body::from_stream(chunks))
            .unwrap()
    }

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let app = axum::Router::new().route("/v1/chat/completions", post(stream));
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    address
}

pub(super) async fn spawn_incomplete_chat_completion_sse_upstream() -> SocketAddr {
    async fn stream() -> Response<Body> {
        let chunks = stream::iter([Ok::<_, std::io::Error>(Bytes::from_static(
            b"data: {\"id\":\"chatcmpl_stream\",\"object\":\"chat.completion.chunk\",\"created\":1,\"model\":\"gpt-4.1\",\"choices\":[{\"index\":0,\"delta\":{\"role\":\"assistant\",\"content\":\"partial\"},\"finish_reason\":null}]}\n\n",
        ))]);

        Response::builder()
            .status(StatusCode::OK)
            .header("content-type", "text/event-stream")
            .body(Body::from_stream(chunks))
            .unwrap()
    }

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let app = axum::Router::new().route("/v1/chat/completions", post(stream));
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    address
}

pub(super) async fn spawn_anthropic_messages_sse_upstream() -> SocketAddr {
    async fn stream() -> Response<Body> {
        let chunks = stream::iter([
            Ok::<_, std::io::Error>(Bytes::from_static(
                b"event: message_start\n\
data: {\"type\":\"message_start\",\"message\":{\"id\":\"msg_stream\",\"type\":\"message\",\"role\":\"assistant\",\"model\":\"claude-test\",\"content\":[],\"stop_reason\":null,\"stop_sequence\":null,\"stop_details\":null,\"container\":null,\"usage\":{\"input_tokens\":8,\"output_tokens\":0,\"cache_creation\":null,\"cache_creation_input_tokens\":null,\"cache_read_input_tokens\":null,\"inference_geo\":null,\"server_tool_use\":null,\"service_tier\":\"standard\"}}}\n\n",
            )),
            Ok(Bytes::from_static(
                b"event: content_block_delta\n\
data: {\"type\":\"content_block_delta\",\"index\":0,\"delta\":{\"type\":\"text_delta\",\"text\":\"ok\"}}\n\n",
            )),
            Ok(Bytes::from_static(
                b"event: message_delta\n\
data: {\"type\":\"message_delta\",\"delta\":{\"stop_reason\":\"end_turn\",\"stop_sequence\":null,\"stop_details\":null,\"container\":null},\"usage\":{\"input_tokens\":8,\"output_tokens\":2,\"cache_creation_input_tokens\":null,\"cache_read_input_tokens\":null,\"server_tool_use\":null}}\n\n",
            )),
            Ok(Bytes::from_static(
                b"event: message_stop\n\
data: {\"type\":\"message_stop\"}\n\n",
            )),
        ]);

        Response::builder()
            .status(StatusCode::OK)
            .header("content-type", "text/event-stream")
            .body(Body::from_stream(chunks))
            .unwrap()
    }

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let app = axum::Router::new().route("/v1/messages", post(stream));
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    address
}

pub(super) async fn spawn_anthropic_messages_compat_sse_upstream() -> SocketAddr {
    async fn stream() -> Response<Body> {
        let chunks = stream::iter([
            Ok::<_, std::io::Error>(Bytes::from_static(
                b"event: message_start\n\
data: {\"type\":\"message_start\",\"id\":\"msg_stream\",\"role\":\"assistant\",\"model\":\"glm-5.1\",\"content\":[],\"stop_reason\":null,\"stop_sequence\":null,\"usage\":{\"input_tokens\":8,\"output_tokens\":0}}\n\n",
            )),
            Ok(Bytes::from_static(
                b"event: content_block_start\n\
data: {\"type\":\"content_block_start\",\"index\":0,\"content_block\":{\"type\":\"tool_use\",\"id\":\"toolu_1\",\"name\":\"lookup\",\"input\":{}}}\n\n",
            )),
            Ok(Bytes::from_static(
                b"event: message_delta\n\
data: {\"type\":\"message_delta\",\"delta\":{\"stop_reason\":\"tool_use\",\"stop_sequence\":null},\"usage\":{\"output_tokens\":2,\"server_tool_use\":{\"web_search_requests\":1}}}\n\n",
            )),
            Ok(Bytes::from_static(
                b"event: message_stop\n\
data: {\"type\":\"message_stop\"}\n\n",
            )),
        ]);

        Response::builder()
            .status(StatusCode::OK)
            .header("content-type", "text/event-stream")
            .body(Body::from_stream(chunks))
            .unwrap()
    }

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let app = axum::Router::new().route("/v1/messages", post(stream));
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    address
}

pub(super) async fn spawn_incomplete_anthropic_messages_sse_upstream() -> SocketAddr {
    async fn stream() -> Response<Body> {
        let chunks = stream::iter([Ok::<_, std::io::Error>(Bytes::from_static(
            b"event: message_start\n\
data: {\"type\":\"message_start\",\"message\":{\"id\":\"msg_stream\",\"type\":\"message\",\"role\":\"assistant\",\"model\":\"claude-test\",\"content\":[],\"stop_reason\":null,\"stop_sequence\":null,\"stop_details\":null,\"container\":null,\"usage\":{\"input_tokens\":8,\"output_tokens\":0,\"cache_creation\":null,\"cache_creation_input_tokens\":null,\"cache_read_input_tokens\":null,\"inference_geo\":null,\"server_tool_use\":null,\"service_tier\":\"standard\"}}}\n\n",
        ))]);

        Response::builder()
            .status(StatusCode::OK)
            .header("content-type", "text/event-stream")
            .body(Body::from_stream(chunks))
            .unwrap()
    }

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let app = axum::Router::new().route("/v1/messages", post(stream));
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    address
}

pub(super) async fn spawn_null_session_id_tool_argument_sse_upstream() -> SocketAddr {
    async fn stream() -> Response<Body> {
        let chunks = stream::iter([
            Ok::<_, std::io::Error>(Bytes::from_static(
                b"event: response.function_call_arguments.delta\n\
 data: {\"type\":\"response.function_call_arguments.delta\",\"delta\":\"{\\\"session_id\\\":null,\",\"item_id\":\"fc_spawn\",\"output_index\":0,\"sequence_number\":1}\n\n",
            )),
            Ok(Bytes::from_static(
                b"event: response.function_call_arguments.delta\n\
 data: {\"type\":\"response.function_call_arguments.delta\",\"delta\":\"\\\"message\\\":\\\"hi\\\"}\",\"item_id\":\"fc_spawn\",\"output_index\":0,\"sequence_number\":2}\n\n",
            )),
            Ok(Bytes::from_static(
                b"event: response.function_call_arguments.done\n\
 data: {\"type\":\"response.function_call_arguments.done\",\"item_id\":\"fc_spawn\",\"output_index\":0,\"sequence_number\":3}\n\n",
            )),
            Ok(Bytes::from_static(
                b"event: response.completed\n\
 data: {\"type\":\"response.completed\",\"response\":{}}\n\n",
            )),
        ]);

        Response::builder()
            .status(StatusCode::OK)
            .header("content-type", "text/event-stream")
            .body(Body::from_stream(chunks))
            .unwrap()
    }

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let app = axum::Router::new().route("/v1/responses", post(stream));
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    address
}

pub(super) async fn spawn_empty_session_id_tool_argument_sse_upstream() -> SocketAddr {
    async fn stream() -> Response<Body> {
        let chunks = stream::iter([
            Ok::<_, std::io::Error>(Bytes::from_static(
                b"event: response.function_call_arguments.delta\n\
 data: {\"type\":\"response.function_call_arguments.delta\",\"delta\":\"{\\\"session_id\\\":\\\"\\\",\",\"item_id\":\"fc_spawn\",\"output_index\":0,\"sequence_number\":1}\n\n",
            )),
            Ok(Bytes::from_static(
                b"event: response.function_call_arguments.delta\n\
 data: {\"type\":\"response.function_call_arguments.delta\",\"delta\":\"\\\"message\\\":\\\"hi\\\"}\",\"item_id\":\"fc_spawn\",\"output_index\":0,\"sequence_number\":2}\n\n",
            )),
            Ok(Bytes::from_static(
                b"event: response.function_call_arguments.done\n\
 data: {\"type\":\"response.function_call_arguments.done\",\"item_id\":\"fc_spawn\",\"output_index\":0,\"sequence_number\":3}\n\n",
            )),
            Ok(Bytes::from_static(
                b"event: response.completed\n\
 data: {\"type\":\"response.completed\",\"response\":{}}\n\n",
            )),
        ]);

        Response::builder()
            .status(StatusCode::OK)
            .header("content-type", "text/event-stream")
            .body(Body::from_stream(chunks))
            .unwrap()
    }

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let app = axum::Router::new().route("/v1/responses", post(stream));
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    address
}

pub(super) async fn spawn_complete_sse_upstream() -> SocketAddr {
    async fn complete() -> Response<Body> {
        let chunks = stream::iter([
            Ok::<_, std::io::Error>(Bytes::from_static(
                b"event: response.output_text.delta\ndata: {\"delta\":\"hello\"}\n\n",
            )),
            Ok(Bytes::from_static(
                b"event: response.completed\ndata: {}\n\n",
            )),
        ]);
        Response::builder()
            .status(StatusCode::OK)
            .header("content-type", "text/event-stream")
            .body(Body::from_stream(chunks))
            .unwrap()
    }

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let app = axum::Router::new().route("/v1/responses", post(complete));
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    address
}

pub(super) async fn spawn_incomplete_response_sse_upstream() -> SocketAddr {
    async fn incomplete() -> Response<Body> {
        let chunks = stream::iter([Ok::<_, std::io::Error>(Bytes::from_static(
            b"event: response.output_text.delta\ndata: {\"delta\":\"partial\"}\n\n",
        ))]);
        Response::builder()
            .status(StatusCode::OK)
            .header("content-type", "text/event-stream")
            .body(Body::from_stream(chunks))
            .unwrap()
    }

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let app = axum::Router::new().route("/v1/responses", post(incomplete));
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    address
}

pub(super) async fn spawn_response_metadata_sse_upstream() -> SocketAddr {
    async fn metadata() -> Response<Body> {
        let chunks = stream::iter([
            Ok::<_, std::io::Error>(Bytes::from_static(
                b"event: response.output_item.done\n\
data: {\"type\":\"response.output_item.done\",\"sequence_number\":3,\"item\":{\"id\":\"fc_metadata\",\"type\":\"function_call\",\"name\":\"edit_file\"}}\n\n",
            )),
            Ok(Bytes::from_static(
                b"event: response.completed\n\
data: {\"type\":\"response.completed\",\"sequence_number\":4,\"response\":{\"id\":\"resp_metadata\",\"model\":\"gpt-5.4\",\"status\":\"completed\",\"service_tier\":\"default\",\"usage\":{\"input_tokens\":100,\"output_tokens\":20,\"total_tokens\":120,\"input_tokens_details\":{\"cached_tokens\":80},\"output_tokens_details\":{\"reasoning_tokens\":7}},\"output\":[{\"id\":\"fc_metadata\",\"type\":\"function_call\",\"name\":\"edit_file\"}]}}\n\n",
            )),
        ]);

        Response::builder()
            .status(StatusCode::OK)
            .header("content-type", "text/event-stream")
            .header("x-ratelimit-limit-requests", "10")
            .header("x-ratelimit-remaining-requests", "9")
            .header("x-ratelimit-limit-tokens", "1000")
            .header("x-ratelimit-remaining-tokens", "900")
            .header("x-ratelimit-reset-requests", "1s")
            .header("x-codex-primary-used-percent", "42.5")
            .header("x-codex-primary-reset-after-seconds", "600")
            .header("x-codex-primary-window-minutes", "300")
            .body(Body::from_stream(chunks))
            .unwrap()
    }

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let app = axum::Router::new().route("/v1/responses", post(metadata));
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    address
}

pub(super) async fn spawn_incomplete_tool_argument_sse_upstream(
    capture: Arc<Capture>,
) -> SocketAddr {
    async fn incomplete(State(capture): State<Arc<Capture>>, body: String) -> Response<Body> {
        let payload = serde_json::from_str::<Value>(&body).unwrap();
        capture.payloads.lock().await.push(payload);

        let chunks = stream::iter([Ok::<_, std::io::Error>(Bytes::from_static(
            b"event: response.function_call_arguments.delta\n\
data: {\"type\":\"response.function_call_arguments.delta\",\"delta\":\"{\\\"cmd\\\"\",\"item_id\":\"fc_trimmed\",\"output_index\":0,\"sequence_number\":1}\n\n",
        ))]);

        Response::builder()
            .status(StatusCode::OK)
            .header("content-type", "text/event-stream")
            .body(Body::from_stream(chunks))
            .unwrap()
    }

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let app = axum::Router::new()
        .route("/v1/responses", post(incomplete))
        .with_state(capture);
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    address
}

pub(super) async fn spawn_stalled_tool_argument_sse_upstream() -> SocketAddr {
    async fn stalled() -> Response<Body> {
        let chunks = stream::once(async {
            Ok::<_, std::io::Error>(Bytes::from_static(
                b"event: response.function_call_arguments.delta\n\
data: {\"type\":\"response.function_call_arguments.delta\",\"delta\":\"{\\\"cmd\\\"\",\"item_id\":\"fc_stalled\",\"output_index\":0,\"sequence_number\":1}\n\n",
            ))
        })
        .chain(stream::pending());

        Response::builder()
            .status(StatusCode::OK)
            .header("content-type", "text/event-stream")
            .body(Body::from_stream(chunks))
            .unwrap()
    }

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let app = axum::Router::new().route("/v1/responses", post(stalled));
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    address
}

pub(super) async fn spawn_unicode_sse_upstream() -> SocketAddr {
    async fn unicode() -> Response<Body> {
        let mut chunks = Vec::new();
        for _ in 0..128 {
            chunks.push(Ok::<_, std::io::Error>(Bytes::from(format!(
                "event: response.output_text.delta\n\
data: {{\"type\":\"response.output_text.delta\",\"delta\":\"{}\"}}\n\n",
                "测试".repeat(64)
            ))));
        }
        chunks.push(Ok(Bytes::from_static(
            b"event: response.completed\ndata: {\"type\":\"response.completed\",\"response\":{}}\n\n",
        )));

        Response::builder()
            .status(StatusCode::OK)
            .header("content-type", "text/event-stream")
            .body(Body::from_stream(stream::iter(chunks)))
            .unwrap()
    }

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let app = axum::Router::new().route("/v1/responses", post(unicode));
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    address
}

#[derive(Debug)]
struct ShimOptions {
    provider_protocol: proxai::protocol::ProviderProtocol,
    provider_compatibility: proxai::config::ProviderCompatibility,
    capture_dir: Option<PathBuf>,
    request_capture_enabled: bool,
    upstream_capture_enabled: bool,
    sse_tool_call_timeout: Option<Duration>,
    routes: Vec<proxai::config::RouteConfig>,
    upstream_base_path: String,
}

impl ShimOptions {
    fn new(provider_protocol: proxai::protocol::ProviderProtocol) -> Self {
        Self {
            provider_protocol,
            provider_compatibility: proxai::config::ProviderCompatibility::default(),
            capture_dir: None,
            request_capture_enabled: false,
            upstream_capture_enabled: false,
            sse_tool_call_timeout: None,
            routes: Vec::new(),
            upstream_base_path: String::new(),
        }
    }
}

pub(super) async fn spawn_shim(upstream_address: SocketAddr) -> SocketAddr {
    spawn_shim_with_base_path(
        upstream_address,
        proxai::protocol::ProviderProtocol::OpenaiResponses,
        "",
    )
    .await
}

pub(super) async fn spawn_shim_with_base_path(
    upstream_address: SocketAddr,
    provider_protocol: proxai::protocol::ProviderProtocol,
    base_path: &str,
) -> SocketAddr {
    let mut options = ShimOptions::new(provider_protocol);
    options.upstream_base_path = base_path.to_string();
    spawn_shim_with_options(upstream_address, options).await
}

pub(super) async fn spawn_chat_shim(upstream_address: SocketAddr) -> SocketAddr {
    spawn_shim_with_capture_options(
        upstream_address,
        proxai::protocol::ProviderProtocol::OpenaiChatCompletions,
        None,
        false,
        false,
        None,
    )
    .await
}

pub(super) async fn spawn_anthropic_shim(upstream_address: SocketAddr) -> SocketAddr {
    spawn_shim_with_capture_options(
        upstream_address,
        proxai::protocol::ProviderProtocol::AnthropicMessages,
        None,
        false,
        false,
        None,
    )
    .await
}

pub(super) async fn spawn_anthropic_shim_with_model_route(
    upstream_address: SocketAddr,
) -> SocketAddr {
    spawn_shim_with_routes(
        upstream_address,
        proxai::protocol::ProviderProtocol::AnthropicMessages,
        vec![proxai::config::RouteConfig {
            request_protocol: Some(proxai::protocol::RequestProtocol::AnthropicMessages),
            match_kind: proxai::config::MatchKind::Exact,
            model_pattern: "claude-request".to_string(),
            provider_name: "openai_default".to_string(),
            upstream_model: Some("claude-upstream".to_string()),
        }],
    )
    .await
}

pub(super) async fn spawn_responses_to_anthropic_shim(upstream_address: SocketAddr) -> SocketAddr {
    spawn_shim_with_routes(
        upstream_address,
        proxai::protocol::ProviderProtocol::AnthropicMessages,
        vec![proxai::config::RouteConfig {
            request_protocol: Some(proxai::protocol::RequestProtocol::OpenaiResponses),
            match_kind: proxai::config::MatchKind::Glob,
            model_pattern: "*".to_string(),
            provider_name: "openai_default".to_string(),
            upstream_model: Some("claude-upstream".to_string()),
        }],
    )
    .await
}

pub(super) async fn spawn_anthropic_to_responses_shim(upstream_address: SocketAddr) -> SocketAddr {
    spawn_shim_with_routes(
        upstream_address,
        proxai::protocol::ProviderProtocol::OpenaiResponses,
        vec![proxai::config::RouteConfig {
            request_protocol: Some(proxai::protocol::RequestProtocol::AnthropicMessages),
            match_kind: proxai::config::MatchKind::Glob,
            model_pattern: "*".to_string(),
            provider_name: "openai_default".to_string(),
            upstream_model: Some("gpt-upstream".to_string()),
        }],
    )
    .await
}

pub(super) async fn spawn_shim_with_capture(
    upstream_address: SocketAddr,
    capture_dir: Option<PathBuf>,
) -> SocketAddr {
    spawn_shim_with_capture_options(
        upstream_address,
        proxai::protocol::ProviderProtocol::OpenaiResponses,
        capture_dir,
        true,
        false,
        None,
    )
    .await
}

pub(super) async fn spawn_shim_with_capture_and_timeout(
    upstream_address: SocketAddr,
    capture_dir: Option<PathBuf>,
    sse_tool_call_timeout: Option<Duration>,
) -> SocketAddr {
    let request_capture_enabled = capture_dir.is_some();
    spawn_shim_with_capture_options(
        upstream_address,
        proxai::protocol::ProviderProtocol::OpenaiResponses,
        capture_dir,
        request_capture_enabled,
        false,
        sse_tool_call_timeout,
    )
    .await
}

pub(super) async fn spawn_shim_with_capture_options(
    upstream_address: SocketAddr,
    provider_protocol: proxai::protocol::ProviderProtocol,
    capture_dir: Option<PathBuf>,
    request_capture_enabled: bool,
    upstream_capture_enabled: bool,
    sse_tool_call_timeout: Option<Duration>,
) -> SocketAddr {
    let mut options = ShimOptions::new(provider_protocol);
    options.capture_dir = capture_dir;
    options.request_capture_enabled = request_capture_enabled;
    options.upstream_capture_enabled = upstream_capture_enabled;
    options.sse_tool_call_timeout = sse_tool_call_timeout;
    spawn_shim_with_options(upstream_address, options).await
}

async fn spawn_shim_with_routes(
    upstream_address: SocketAddr,
    provider_protocol: proxai::protocol::ProviderProtocol,
    routes: Vec<proxai::config::RouteConfig>,
) -> SocketAddr {
    let mut options = ShimOptions::new(provider_protocol);
    options.routes = routes;
    spawn_shim_with_options(upstream_address, options).await
}

pub(super) async fn spawn_anthropic_strict_shim(upstream_address: SocketAddr) -> SocketAddr {
    let mut options = ShimOptions::new(proxai::protocol::ProviderProtocol::AnthropicMessages);
    options.provider_compatibility = proxai::config::ProviderCompatibility::Strict;
    spawn_shim_with_options(upstream_address, options).await
}

async fn spawn_shim_with_options(upstream_address: SocketAddr, options: ShimOptions) -> SocketAddr {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let mut providers = std::collections::BTreeMap::new();
    providers.insert(
        "openai_default".to_string(),
        proxai::config::ProviderConfig {
            protocol: options.provider_protocol,
            base_url: url::Url::parse(&format!(
                "http://{upstream_address}{}",
                options.upstream_base_path
            ))
            .unwrap(),
            api_key: "test-upstream-key".to_string(),
            compatibility: options.provider_compatibility,
            read_idle_timeout: Duration::from_secs(120),
        },
    );
    let state = AppState::new(
        proxai::config::DefaultProviderNamesConfig {
            openai_responses: "openai_default".to_string(),
            openai_chat_completions: "openai_default".to_string(),
            anthropic_messages: "openai_default".to_string(),
        },
        providers,
        options.routes,
    )
    .unwrap()
    .with_capture_dir(options.capture_dir)
    .with_capture_config(proxai::config::CaptureConfig {
        inbound_request_enabled: options.request_capture_enabled,
        forwarded_request_enabled: options.request_capture_enabled,
        upstream_response_enabled: options.upstream_capture_enabled,
        outbound_response_enabled: false,
    })
    .with_sse_tool_call_timeout(options.sse_tool_call_timeout);
    tokio::spawn(async move {
        state.serve(listener, std::future::pending()).await.unwrap();
    });
    address
}

pub(super) fn unique_capture_dir() -> PathBuf {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let counter = CAPTURE_DIR_COUNTER.fetch_add(1, Ordering::Relaxed);
    std::env::current_dir()
        .unwrap()
        .join("target")
        .join("test-captures")
        .join(format!("capture-{timestamp}-{counter}"))
}

pub(super) async fn capture_files(capture_dir: &Path) -> Vec<PathBuf> {
    let mut reader = fs::read_dir(capture_dir).await.unwrap();
    let mut files = Vec::new();
    while let Some(entry) = reader.next_entry().await.unwrap() {
        files.push(entry.path());
    }
    files.sort();
    files
}

pub(super) async fn read_json_file(files: &[PathBuf], kind: &str) -> Value {
    let path = files
        .iter()
        .find(|path| path.to_string_lossy().contains(kind))
        .unwrap();
    serde_json::from_slice(&fs::read(path).await.unwrap()).unwrap()
}

pub(super) async fn read_text_file(files: &[PathBuf], kind: &str) -> String {
    let path = files
        .iter()
        .find(|path| path.to_string_lossy().contains(kind))
        .unwrap();
    String::from_utf8(fs::read(path).await.unwrap()).unwrap()
}
