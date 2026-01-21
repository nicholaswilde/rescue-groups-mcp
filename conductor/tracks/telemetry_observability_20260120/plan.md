# Plan: Telemetry & Observability

## Goal
Enhance the observability of the MCP server by implementing structured JSON logging and HTTP request tracing. This allows for better debugging and monitoring in production environments.

## Tasks

### 1. Update Dependencies
- [x] Check `Cargo.toml`. `tracing` and `tracing-subscriber` are present.
- [x] Ensure `tracing-subscriber` has `json` feature enabled.
- [x] Add `tower-http` with `trace` feature for HTTP request logging.

### 2. Structured Logging
- [x] Modify `src/main.rs` logging initialization.
- [x] Check `RUST_LOG_FORMAT` env var. If "json", use `tracing_subscriber::fmt::format().json()`.
- [x] Otherwise default to "pretty" or "compact".

### 3. HTTP Request Tracing
- [x] Modify `src/server.rs`.
- [x] Add `tower_http::trace::TraceLayer` to the Axum router.
- [x] Configure it to log request start/end, method, URI, status, and latency.

### 4. Verification
- [x] Run `cargo check`.
- [x] Run `RUST_LOG_FORMAT=json cargo run -- server`.
- [x] Verify logs are in JSON format.
- [x] Verified default logging format.

## Outcome
Implemented structured JSON logging (via `RUST_LOG_FORMAT=json`) and HTTP request tracing (via `TraceLayer`).