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

    let app = Router::new()
        .route("/", post(http_handler))
        .route("/sse", get(sse_handler))
        .route("/message", post(message_handler))
        .layer(TraceLayer::new_for_http())
        .with_state(app_state);

    let addr: SocketAddr = format!("{}:{}", args.host, args.port)
        .parse()
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidInput, e))?;
    info!("RescueGroups MCP Server running (HTTP + SSE) on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await
}

use std::io::{self, BufRead, Write};

pub async fn run_stdio_server(settings: Settings) -> Result<(), std::io::Error> {
    let stdin = io::stdin();
    let mut reader = stdin.lock();
    let mut line = String::new();

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
            println!("{}", output);
            io::stdout().flush()?;
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
