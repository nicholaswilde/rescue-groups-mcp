# Specification: Refine Error Handling and Async Tasks

## Goal
Enhance the reliability and developer experience of the RescueGroups MCP server by refining how errors are reported to clients and how asynchronous tasks are managed internally.

## Scope
- **Error Handling:** Define custom error types, improve mapping to JSON-RPC error codes, and enhance logging.
- **Async Management:** Improve the handling of long-running operations, implement timeouts, and ensure robust task lifecycle management.

## Requirements

### Error Handling
1. **Custom Error Types:** Create a robust error enum (e.g., `AppError`) that covers API errors, configuration issues, network failures, and internal logic errors.
2. **JSON-RPC Mapping:** Ensure all internal errors are correctly mapped to standard JSON-RPC 2.0 error codes (`-32700` to `-32000`) or meaningful application-defined codes.
3. **Structured Logging:** Use the `tracing` or `log` crate to provide detailed error context for debugging without leaking sensitive information to the client.

### Async Task Management
1. **Task Lifecycle:** Use modern Tokio patterns (like `JoinSet`) to manage groups of related async tasks.
2. **Timeouts:** Ensure all external API requests have appropriate timeouts to prevent the MCP server from hanging.
3. **Graceful Shutdown:** (Optional) Improve how the server handles shutdown signals to ensure pending tasks are completed or cancelled cleanly.

## Design
- Refactor the current `Result<..., Box<dyn Error>>` usage to a more specific error type.
- Implement `From<T>` for various error types (e.g., `reqwest::Error`, `serde_json::Error`) to streamline error propagation.
- Update MCP tool handlers to return these refined errors.
