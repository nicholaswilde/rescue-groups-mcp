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
