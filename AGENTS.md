# Project Context: rescue-groups-mcp

## Project Overview
This project is a **Rust implementation of a RescueGroups MCP (Model Context Protocol) server**. It allows users to search for adoptable pets, view animal details, and find rescue organizations using the RescueGroups.org API.

**Current Status:** Functional Rust implementation with core MCP tools, caching, configuration file support, and comprehensive unit tests.
**Implemented Features:**
*   **Search & Listing:** Adoptable pets, species, breeds, and organizations.
*   **Detailed Information:** Animal details, contact info, and organization profiles.
*   **Comparison:** Side-by-side comparison of up to 5 animals.
*   **Caching:** High-performance asynchronous caching layer using `moka` (15-minute TTL).
*   **Language:** Rust.
*   **Transport:** Stdio (JSON-RPC 2.0) and HTTP (SSE/POST).

## Key Files
*   `README.md`: User documentation and tool list.
*   `Cargo.toml`: Project dependencies.
*   `Taskfile.yml`: Task definitions for building, testing, and linting.
*   `src/main.rs`: Main application logic, MCP server implementation, and API client.
*   `build.rs`: Dynamic versioning script.
*   `Dockerfile`: Docker build configuration.
*   `.github/workflows/ci.yml`: GitHub Actions CI workflow.

## Building and Running
1.  **Build:** `cargo build --release`
2.  **Run (Stdio):** `./target/release/rescue-groups-mcp` (requires API key via env or config)
3.  **Run (HTTP):** `./target/release/rescue-groups-mcp http --host 0.0.0.0 --port 3000`
4.  **Test:** `cargo test` or `task test`

### TODOs
*   Refine async task handling for long-running operations.
*   Enhance error reporting for MCP clients.
*   Implement "lazy loading" of functions to reduce AI tokens usage.

## Development Conventions
*   **Language:** Rust
*   **Style:** Standard Rust formatting (`cargo fmt`) and linting (`cargo clippy`) are expected.
*   **Post-Implementation:** Always run `cargo fmt` and `cargo clippy --workspace --all-targets --all-features -- -D warnings` (or `task lint`) after adding new functions to ensure code quality.
*   **CI Checks:** Run `task test` to execute formatting and tests.
*   **Documentation:** All functions and tools must be documented in the `README.md`.
*   **Testing:** Every function and MCP tool must have corresponding unit tests in `src/main.rs` (or relevant module).
*   **Versioning:** Version is dynamically handled by `build.rs` via git tags. `Cargo.toml` version should be bumped manually when creating a new release tag.

## Live Testing Workflow
When asked to test new functions since the last tag against a live environment:

1.  **Identify Changes:** Review `src/main.rs` to see which new tools were added.
2.  **Verify Config:** Ensure `RESCUE_GROUPS_API_KEY` is set or a `config.toml` exists.
3.  **Build:** Run `cargo build --release` or `cargo run`.
4.  **CLI Interaction:** Use the built-in CLI commands to verify tool functionality (e.g., `cargo run -- get-animal --animal-id <ID>`).

## Release Summary Guidelines
*   When asked for a GitHub release summary from the previous git tag to the current one, only summarize the MCP server functionality. Chore and documentation updates should be excluded.
*   Add emoji when appropriate to the release summary.

## Gemini Added Memories
- Fixed the `_jsonrpc` deserialization issue in `src/main.rs` by adding `#[serde(rename = "jsonrpc")]`.
- Implemented a suite of 8 MCP tools: `search_adoptable_pets`, `list_animals`, `get_animal_details`, `list_species`, `list_breeds`, `search_organizations`, `get_organization_details`, and `list_org_animals`.
- Added a high-performance asynchronous caching layer using `moka` with a 15-minute TTL to respect RescueGroups API rate limits.
- Refactored the codebase for better testability by introducing a `base_url` in the `Settings` struct.
- Implemented a comprehensive test suite with 12 unit tests using `mockito` to mock API responses and verify both logic and network interactions.
- Updated `README.md` with detailed feature lists, tool descriptions, and build/test instructions.
- Implemented `compare_animals` tool to allow side-by-side comparison of up to 5 animals, including a Markdown table formatter.
- Implemented `get_contact_info` tool which uses the `?include=orgs` API parameter to retrieve animal-specific rescue contact details (Email, Phone, Org website).
- Fixed `Send + Sync` trait bound issues in `axum` handlers by updating `Box<dyn Error>` to `Box<dyn Error + Send + Sync>` globally.
- Fixed Docker build error "Missing dependency: cmake" by installing `cmake` in the `Dockerfile`, required for `aws-lc-sys` compilation.
- Fixed API response handling for single items wrapped in arrays (e.g., `get_animal_details`).
- Enhanced `list_breeds` to dynamically resolve species names (e.g., "dogs") to IDs.
- Implemented `build.rs` for dynamic versioning using `git describe`.
- Verified all read-only MCP tools against live RescueGroups API (2026-01-18) using `cargo run`. Confirmed functionality for search, details, organization, and metadata tools.