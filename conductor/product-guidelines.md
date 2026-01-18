# Product Guidelines: RescueGroups MCP Server

## Documentation & Response Style
- **Clarity First:** Technical documentation and tool descriptions must be clear, unambiguous, and easy to follow.
- **Concise & Actionable:** Responses should provide the necessary information without unnecessary filler, prioritizing user efficiency.
- **Professional Tone:** Maintain a professional and helpful tone, acknowledging the impact of the rescue mission where appropriate.
- **Markdown Excellence:** Use consistent Markdown formatting for readability, including lists, tables, and code blocks.

## Visual Identity (Markdown Responses)
- **Emoji Usage:** Use animal and technology-related emojis sparingly to add personality and visual cues to responses (e.g., 🐕, 🐈, 🤖).
- **Structured Data:** Present complex animal comparisons or metadata lists using Markdown tables for easy scanning.
- **Image Integration:** Ensure animal photos are clearly linked or embedded as Markdown images to enhance the emotional connection.

## Development Principles
- **Safety & Privacy:** Never expose API keys or sensitive user data in logs or responses.
- **Error Handling:** Provide informative error messages that help users or agents resolve issues (e.g., invalid postal code, API rate limits).
- **Performance Aware:** Always consider the latency impact of API calls and rely on the caching layer whenever possible.
- **Extensibility:** Design tools and modules to be easily extendable as the RescueGroups.org API evolves.

## Development Conventions
- **Language:** Rust
- **Style:** Standard Rust formatting (`cargo fmt`) and linting (`cargo clippy`) are expected.
- **Post-Implementation:** Always run `cargo fmt` and `cargo clippy --workspace --all-targets --all-features -- -D warnings` (or `task lint`) after adding new functions to ensure code quality.
- **CI Checks:** Run `task test:ci` to execute formatting and tests.
- **Documentation:** All functions and tools must be documented in the `README.md`.
- **Testing:** Every function and MCP tool must have corresponding unit tests in `src/main.rs` (or relevant module).
- **Versioning:** Version is dynamically handled by `build.rs` via git tags. `Cargo.toml` version should be bumped manually when creating a new release tag.
- **Release Summary:** When asked for a GitHub release summary, only summarize MCP server functionality. Add emoji when appropriate.

