# Tech Stack: RescueGroups MCP Server

## Core Language & Runtime
- **Language:** [Rust](https://www.rust-lang.org/) (Edition 2021) - Chosen for performance, memory safety, and excellent concurrency support.
- **Async Runtime:** [Tokio](https://tokio.rs/) - The industry-standard asynchronous runtime for Rust.

## API & Networking
- **API Client:** [Reqwest](https://docs.rs/reqwest/) - High-level HTTP client for making requests to RescueGroups.org.
- **Web Framework:** [Axum](https://docs.rs/axum/) - Used to provide the HTTP transport (SSE/POST) for the MCP server.
- **Transports:** JSON-RPC 2.0 over Stdio and HTTP.

## Data Handling
- **Serialization/Deserialization:** [Serde](https://serde.rs/) - Powerful framework for handling JSON, YAML, and TOML.
- **Caching:** [Moka](https://docs.rs/moka/) - High-performance asynchronous caching to respect API rate limits and improve response times.

## CLI & Configuration
- **CLI Framework:** [Clap](https://docs.rs/clap/) (v4) - For robust command-line argument parsing and help generation.
- **Configuration:** Support for `config.toml`, `config.yaml`, and `config.json` via Serde.

## Development & Testing
- **Testing:** Standard Rust `cargo test` suite.
- **API Mocking:** [Mockito](https://docs.rs/mockito/) - Used for reliable and reproducible unit tests by mocking RescueGroups.org API responses.
- **Build System:** [Task](https://taskfile.dev/) - For automated linting, testing, and building workflows.
- **Versioning:** Dynamic versioning via `build.rs` using `git describe`.

## Infrastructure
- **Containerization:** [Docker](https://www.docker.com/) & [Docker Compose](https://docs.docker.com/compose/) - For consistent deployment and local development.
- **CI/CD:** GitHub Actions for automated testing and releases.
