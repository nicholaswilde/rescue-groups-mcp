use crate::cli::HttpArgs;
use crate::config::Settings;
use crate::mcp::{format_json_rpc_response, process_mcp_request, JsonRpcRequest};
use axum::{
    extract::{Json, Query, State},
    http::{HeaderMap, StatusCode},
    response::{
        sse::{Event, KeepAlive, Sse},
        IntoResponse,
    },
    routing::{get, post},
    Router,
};
use futures::stream::Stream;
use serde::Deserialize;
use std::collections::HashMap;
use std::convert::Infallible;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use tokio_stream::wrappers::UnboundedReceiverStream;
use tower_http::trace::TraceLayer;
use tracing::{debug, info, warn};
use uuid::Uuid;

pub type SessionSender = mpsc::UnboundedSender<Result<Event, Infallible>>;
pub type SessionsMap = Arc<RwLock<HashMap<String, SessionSender>>>;

#[derive(Clone)]
pub struct AppState {
    pub settings: Settings,
    pub auth_token: Option<String>,
    pub sessions: SessionsMap,
}

#[derive(Deserialize)]
pub struct MessageParams {
    session_id: String,
}

pub async fn run_http_server(args: HttpArgs, settings: Settings) -> Result<(), std::io::Error> {
    let app_state = Arc::new(AppState {
        settings,
        auth_token: args.auth_token,
        sessions: Arc::new(RwLock::new(HashMap::new())),
    });

    let app = create_router(app_state);

    let addr: SocketAddr = format!("{}:{}", args.host, args.port)
        .parse()
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidInput, e))?;
    info!("RescueGroups MCP Server running (HTTP + SSE) on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await
}

pub fn create_router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/", post(http_handler))
        .route("/sse", get(sse_handler))
        .route("/message", post(message_handler))
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}

use std::io::{self, BufRead, Write};

pub async fn run_stdio_server(settings: Settings) -> Result<(), std::io::Error> {
    let stdin = io::stdin();
    let stdout = io::stdout();
    run_stdio_server_with_io(stdin.lock(), stdout.lock(), settings).await
}

pub async fn run_stdio_server_with_io<R, W>(
    reader: R,
    mut writer: W,
    settings: Settings,
) -> Result<(), std::io::Error>
where
    R: io::BufRead,
    W: io::Write,
{
    let mut line = String::new();
    let mut reader = reader;

    info!("RescueGroups MCP Server running (Stdio)...");

    loop {
        line.clear();
        if reader.read_line(&mut line)? == 0 {
            break;
        }

        let req: JsonRpcRequest = match serde_json::from_str::<JsonRpcRequest>(&line) {
            Ok(r) => {
                debug!("Received request: method={}", r.method);
                r
            }
            Err(e) => {
                warn!("Failed to parse JSON-RPC request: {}", e);
                continue;
            }
        };

        let response = process_mcp_request(req, &settings).await;

        if let Some(id) = response.0 {
            let output = format_json_rpc_response(id, response.1);
            writeln!(writer, "{}", output)?;
            writer.flush()?;
        }
    }
    Ok(())
}

pub async fn http_handler(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(req): Json<JsonRpcRequest>,
) -> impl IntoResponse {
    // Auth check
    if let Some(token) = &state.auth_token {
        let auth_header = headers
            .get("Authorization")
            .and_then(|h| h.to_str().ok())
            .unwrap_or("");

        if auth_header != format!("Bearer {}", token) {
            warn!("Unauthorized access attempt");
            return (StatusCode::UNAUTHORIZED, "Unauthorized").into_response();
        }
    }

    debug!("Received HTTP request: method={}", req.method);
    let response = process_mcp_request(req, &state.settings).await;

    if let Some(id) = response.0 {
        let output = format_json_rpc_response(id, response.1);
        Json(output).into_response()
    } else {
        StatusCode::NO_CONTENT.into_response()
    }
}

pub async fn sse_handler(
    State(state): State<Arc<AppState>>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let (tx, rx) = mpsc::unbounded_channel();
    let session_id = Uuid::new_v4().to_string();

    // Send initial endpoint event
    let endpoint_url = format!("/message?session_id={}", session_id);
    let _ = tx.send(Ok(Event::default().event("endpoint").data(endpoint_url)));

    state.sessions.write().await.insert(session_id.clone(), tx);

    let stream = UnboundedReceiverStream::new(rx);
    Sse::new(stream).keep_alive(KeepAlive::default())
}

pub async fn message_handler(
    State(state): State<Arc<AppState>>,
    Query(params): Query<MessageParams>,
    Json(req): Json<JsonRpcRequest>,
) -> impl IntoResponse {
    let response = process_mcp_request(req, &state.settings).await;

    if let Some(id) = response.0 {
        let output = format_json_rpc_response(id, response.1);

        // Find session and send response via SSE
        if let Some(tx) = state.sessions.read().await.get(&params.session_id) {
            let _ = tx.send(Ok(Event::default()
                .event("message")
                .data(output.to_string())));
        }
    }

    StatusCode::ACCEPTED
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Settings;
    use axum::http::{Request, StatusCode};
    use governor::{Quota, RateLimiter};
    use moka::future::Cache;
    use serde_json::json;
    use std::num::NonZeroU32;
    use std::sync::Arc;
    use std::time::Duration;
    use tower::ServiceExt;

    fn get_test_settings() -> Settings {
        Settings {
            api_key: "test_key".to_string(),
            base_url: "http://test.url".to_string(),
            default_postal_code: "00000".to_string(),
            default_miles: 50,
            default_species: "dogs".to_string(),
            timeout: Duration::from_secs(1),
            lazy: false,
            cache: Arc::new(Cache::new(10)),
            limiter: Arc::new(RateLimiter::direct(Quota::per_second(
                NonZeroU32::new(100).unwrap(),
            ))),
        }
    }

    #[tokio::test]
    async fn test_http_handler_no_auth() {
        let state = Arc::new(AppState {
            settings: get_test_settings(),
            auth_token: None,
            sessions: Arc::new(RwLock::new(HashMap::new())),
        });

        let app = Router::new()
            .route("/", post(http_handler))
            .with_state(state);

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/")
                    .header("content-type", "application/json")
                    .body(axum::body::Body::from(
                        serde_json::to_string(&json!({
                            "jsonrpc": "2.0",
                            "id": 1,
                            "method": "ping"
                        }))
                        .unwrap(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_http_handler_unauthorized() {
        let state = Arc::new(AppState {
            settings: get_test_settings(),
            auth_token: Some("secret".to_string()),
            sessions: Arc::new(RwLock::new(HashMap::new())),
        });

        let app = Router::new()
            .route("/", post(http_handler))
            .with_state(state);

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/")
                    .header("content-type", "application/json")
                    .body(axum::body::Body::from(
                        serde_json::to_string(&json!({
                            "jsonrpc": "2.0",
                            "id": 1,
                            "method": "ping"
                        }))
                        .unwrap(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn test_sse_handler() {
        let state = Arc::new(AppState {
            settings: get_test_settings(),
            auth_token: None,
            sessions: Arc::new(RwLock::new(HashMap::new())),
        });

        let app = Router::new()
            .route("/sse", get(sse_handler))
            .with_state(state.clone());

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/sse")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(state.sessions.read().await.len(), 1);
    }

    #[tokio::test]
    async fn test_run_stdio_server_with_io() {
        let input = serde_json::to_string(&json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "ping"
        }))
        .unwrap()
            + "\n";
        let mut reader = io::Cursor::new(input);
        let mut writer = Vec::new();
        let settings = get_test_settings();

        let res = run_stdio_server_with_io(&mut reader, &mut writer, settings).await;
        assert!(res.is_ok());
        let output = String::from_utf8(writer).unwrap();
        assert!(output.contains("jsonrpc"));
    }

    #[tokio::test]
    async fn test_run_stdio_server_invalid_json() {
        let input = "invalid\n";
        let mut reader = io::Cursor::new(input);
        let mut writer = Vec::new();
        let settings = get_test_settings();

        let res = run_stdio_server_with_io(&mut reader, &mut writer, settings).await;
        assert!(res.is_ok());
        assert!(writer.is_empty());
    }

    #[tokio::test]
    async fn test_http_handler_no_id() {
        let state = Arc::new(AppState {
            settings: get_test_settings(),
            auth_token: None,
            sessions: Arc::new(RwLock::new(HashMap::new())),
        });

        let app = Router::new().route("/", post(http_handler)).with_state(state);

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/")
                    .header("content-type", "application/json")
                    .body(axum::body::Body::from(
                        serde_json::to_string(&json!({
                            "jsonrpc": "2.0",
                            "method": "notifications/initialized"
                        }))
                        .unwrap(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NO_CONTENT);
    }

    #[tokio::test]
    async fn test_message_handler() {
        let state = Arc::new(AppState {
            settings: get_test_settings(),
            auth_token: None,
            sessions: Arc::new(RwLock::new(HashMap::new())),
        });

        let (tx, mut rx) = mpsc::unbounded_channel();
        let session_id = "test-session".to_string();
        state.sessions.write().await.insert(session_id.clone(), tx);

        let app = Router::new()
            .route("/message", post(message_handler))
            .with_state(state);

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(&format!("/message?session_id={}", session_id))
                    .header("content-type", "application/json")
                    .body(axum::body::Body::from(
                        serde_json::to_string(&json!({
                            "jsonrpc": "2.0",
                            "id": 1,
                            "method": "ping"
                        }))
                        .unwrap(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::ACCEPTED);

        // Check if message was sent to SSE
        let msg = rx.recv().await.unwrap();
        assert!(msg.is_ok());
    }

    #[tokio::test]
    async fn test_message_handler_no_id() {
        let state = Arc::new(AppState {
            settings: get_test_settings(),
            auth_token: None,
            sessions: Arc::new(RwLock::new(HashMap::new())),
        });

        let app = Router::new()
            .route("/message", post(message_handler))
            .with_state(state);

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/message?session_id=test")
                    .header("content-type", "application/json")
                    .body(axum::body::Body::from(
                        serde_json::to_string(&json!({
                            "jsonrpc": "2.0",
                            "method": "notifications/initialized"
                        }))
                        .unwrap(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::ACCEPTED);
    }

    #[tokio::test]
    async fn test_message_handler_invalid_session() {
        let state = Arc::new(AppState {
            settings: get_test_settings(),
            auth_token: None,
            sessions: Arc::new(RwLock::new(HashMap::new())),
        });

        let app = Router::new()
            .route("/message", post(message_handler))
            .with_state(state);

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/message?session_id=invalid")
                    .header("content-type", "application/json")
                    .body(axum::body::Body::from(
                        serde_json::to_string(&json!({
                            "jsonrpc": "2.0",
                            "id": 1,
                            "method": "ping"
                        }))
                        .unwrap(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::ACCEPTED);
    }

    #[tokio::test]
    async fn test_create_router() {
        let state = Arc::new(AppState {
            settings: get_test_settings(),
            auth_token: None,
            sessions: Arc::new(RwLock::new(HashMap::new())),
        });
        let _router = create_router(state);
    }

    #[tokio::test]
    async fn test_run_http_server_startup() {
        let settings = get_test_settings();
        let args = HttpArgs {
            host: "127.0.0.1".to_string(),
            port: 0, // Let OS pick a free port
            auth_token: None,
        };

        let handle = tokio::spawn(async move {
            let _ = run_http_server(args, settings).await;
        });

        tokio::time::sleep(Duration::from_millis(200)).await;
        handle.abort();
    }
}
