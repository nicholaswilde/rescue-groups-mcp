# Plan: Unit Test Coverage

## Goal
Increase the unit test coverage of the project to ensure stability and correctness of core logic, especially after the recent refactoring. The target is to cover `config.rs`, `mcp.rs`, and `cli.rs` logic.

## Tasks

### 1. Config Tests (`src/config.rs`)
- [x] Test `merge_configuration` logic.
    - Verified `cli_key` override logic.
    - Verified error handling for missing API key.

### 2. MCP Tests (`src/mcp.rs`)
- [x] Test `format_json_rpc_response`.
    - Verified success structure (id, result).
    - Verified error structure (id, error code).
- [x] Test `process_mcp_request`.
    - Verified `initialize` response.
    - Verified `tools/list` response contains expected tools.

### 3. CLI Tests (`src/cli.rs`)
- [x] Test `Cli::parse` using `try_parse_from`.
    - Verified argument parsing for `search` with new filters (`color`).
    - Verified `search-events` command fails to parse (removed).

## Outcome
Added comprehensive unit tests for core modules. `cargo test` now runs 9 tests, covering CLI parsing, config merging, JSON-RPC formatting, and output formatting logic.