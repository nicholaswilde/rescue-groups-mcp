# :dog: RescueGroups MCP Server :robot:

[![task](https://img.shields.io/badge/Task-Enabled-brightgreen?style=for-the-badge&logo=task&logoColor=white)](https://taskfile.dev/#/)
[![ci](https://img.shields.io/github/actions/workflow/status/nicholaswilde/rescue-groups-mcp/ci.yml?label=ci&style=for-the-badge&branch=main)](https://github.com/nicholaswilde/rescue-groups-mcp/actions/workflows/ci.yml)
[![coverage](https://img.shields.io/coveralls/github/nicholaswilde/rescue-groups-mcp?style=for-the-badge)](https://coveralls.io/github/nicholaswilde/rescue-groups-mcp)

> [!WARNING]
> This project is currently in active development (v0.x.x) and is **not production-ready**. Features may change, and breaking changes may occur without notice.

An [MCP server](https://modelcontextprotocol.io/docs/getting-started/intro) to interface with [RescueGroups][3] written in Rust.

You will need to request an [API key][1] from the group.

## :sparkles: Features

- **MCP Integration**: Fully compatible with the Model Context Protocol for use with LLMs like Claude.
- **Advanced Caching**: Built-in asynchronous caching (15-minute TTL) using `moka` to reduce API load and stay within rate limits.
- **Multiple Config Formats**: Support for TOML, YAML, and JSON configuration files.
- **Rich Results**: Returns Markdown-formatted animal profiles with embedded images and detailed descriptions.
- **Observability**: Structured JSON logging and HTTP request tracing for production monitoring.
- **Robustness**: Comprehensive unit and integration test suite with mocked API responses.

## :toolbox: MCP Tools

### :mag: Search & Discovery
- `search_adoptable_pets`: Find pets near you by species, postal code, and radius.
    - **Filters**: `good_with_children`, `good_with_dogs`, `good_with_cats`, `house_trained`, `special_needs`, `needs_foster`.
    - **Attributes**: `color`, `pattern` (Partial match).
    - **Sorting**: Sort by `Newest`, `Distance`, or `Random`.
- `list_animals`: Browse the most recent adoptable animals available globally.
- `get_random_pet`: Discover a random adoptable animal for inspiration.
- `search_organizations`: Find animal rescue organizations by location or name.

### :information_source: Details & Profiles
- `get_animal_details`: Fetch a complete profile for a specific animal (description, sex, age, size, and photos).
- `get_contact_info`: Get the primary contact method (email, phone, organization) for a specific animal.
- `get_organization_details`: Fetch a complete profile for a specific organization (mission, address, and contact info).
- `list_org_animals`: List all animals available for adoption at a specific shelter.
- `list_adopted_animals`: List recently adopted animals (Success Stories) to see happy endings near you.

### :bar_chart: Comparison
- `compare_animals`: Compare up to 5 animals side-by-side (Age, Breed, Size, Compatibility).

### :books: Metadata & Reference
- `list_species`: List all animal species supported by the API (e.g., Dog, Cat, Horse).
- `list_breeds`: Discover available breeds for a specific species to refine your searches.
- `list_metadata`: List valid metadata values for animal attributes (colors, patterns, qualities).
- `list_metadata_types`: List all valid metadata categories available for discovery.

### :tools: Utility
- `inspect_tool`: Discover available tools or get detailed schema for a specific tool.

## :bar_chart: Code Coverage

This project uses `cargo-llvm-cov` for code coverage. Coverage reports are manually uploaded to Coveralls.

### Prerequisites

- `cargo-llvm-cov`: Install with `cargo install cargo-llvm-cov`
- `coveralls` binary: Ensure `/usr/local/bin/coveralls` is available.

### Commands

- **Generate HTML Report**: `task test:coverage` (outputs to `target/llvm-cov/html/index.html`)
- **Generate Summary**: `task coverage` (fails if coverage is below 90%)
- **Upload to Coveralls**:
    1. Create a `.env` file with `COVERALLS_REPO_TOKEN=your_token`.
    2. Run `task coverage:report` to generate the LCOV file.
    3. Run `task coverage:upload` to upload to Coveralls.

## :error: Error Handling

The server implements robust error handling and propagates meaningful messages back to the client via JSON-RPC:

- **Validation Errors (-32602)**: Raised when tool arguments are invalid or missing.
- **Resource Not Found (-32004)**: Raised when a specific animal, organization, or tool is not found.
- **API/Network Errors (-32005)**: Raised when there are issues communicating with the RescueGroups API or when the API returns an error status.
- **Internal Errors (-32603)**: General server-side failures (IO, serialization, configuration).

All errors are logged to `stderr` using the `tracing` framework for easy troubleshooting in containerized environments.

## :hammer_and_wrench: Build & Test

To build the project:

```bash
cargo build --release
```

To run the test suite:

```bash
cargo test
```

## :rocket: Usage

### :computer: CLI Mode

The application can be used directly from the command line for quick searches and debugging.

```bash
# Search for cats near 90210
./target/release/rescue-groups-mcp search --species cats --postal-code 90210

# Search for black dogs
./target/release/rescue-groups-mcp search --species dogs --color Black

# Get contact info for an animal
./target/release/rescue-groups-mcp get-contact --animal-id 1234

# Compare multiple animals by ID
./target/release/rescue-groups-mcp compare --animal-ids 1234,5678

# Search for organizations near 90210
./target/release/rescue-groups-mcp search-orgs --postal-code 90210 --miles 25

# List animals at a specific organization
./target/release/rescue-groups-mcp list-org-animals --org-id 123

# List recently adopted dogs (Success Stories)
./target/release/rescue-groups-mcp list-adopted --species dogs --postal-code 90210

# Discover breeds for cats
./target/release/rescue-groups-mcp list-breeds --species cats

# List valid colors metadata
./target/release/rescue-groups-mcp list-metadata --metadata-type colors

# Get raw JSON output (useful for scripting with jq)
./target/release/rescue-groups-mcp search --species cats --json | jq .

# List available species
./target/release/rescue-groups-mcp list-species

# Start the MCP server (default behavior)
./target/release/rescue-groups-mcp server

# Start the MCP server in HTTP mode
./target/release/rescue-groups-mcp http --port 3000 --auth-token mysecrettoken
```

### :shell: Shell Completion

Generate shell completion scripts for your favorite shell.

#### Bash
Add this to your `~/.bashrc`:
```bash
source <(rescue-groups-mcp generate --shell bash)
```

#### Zsh
Add this to your `~/.zshrc`:
```zsh
source <(rescue-groups-mcp generate --shell zsh)
```

#### Fish
Add this to your `~/.config/fish/config.fish`:
```fish
rescue-groups-mcp generate --shell fish | source
```

### :page_facing_up: Man Pages

Generate and view the manual page for the CLI.

```bash
# Generate to a directory
./target/release/rescue-groups-mcp generate --man ./man

# View the generated page
man ./man/rescue-groups-mcp.1
```

### :whale: Docker

You can run the server using Docker or Docker Compose.

#### Using Docker Compose

1.  Configure your API key in `compose.yaml`.
2.  Run the container:

```bash
docker compose up -d
```

#### Using Docker CLI

Build the image:

```bash
docker build -t rescue-groups-mcp .
```

Run the container (MCP Mode):

```bash
docker run -i --rm -e RESCUE_GROUPS_API_KEY=your_key rescue-groups-mcp
```

Run the container (HTTP Mode):

```bash
docker run -d -p 3000:3000 -e RESCUE_GROUPS_API_KEY=your_key rescue-groups-mcp http --port 3000
```

### :speech_balloon: MCP Server Mode

To usage with an LLM, simply run the binary without arguments (or with `server`). It will listen on Stdio for JSON-RPC messages.

### :config: Client Configuration

#### Claude Desktop
Add this to your `claude_desktop_config.json`:

```json
{
  "mcpServers": {
    "rescue-groups": {
      "command": "/absolute/path/to/target/release/rescue-groups-mcp",
      "args": ["server"],
      "env": {
        "RESCUE_GROUPS_API_KEY": "your_api_key_here"
      }
    }
  }
}
```

#### Claude CLI (Claude Code)
Add the server using the `claude` command:

```bash
claude mcp add rescue-groups-mcp -- server --env RESCUE_GROUPS_API_KEY=your_api_key_here
```

### :gear: Configuration File

The server can load configuration from a file named `config.toml`, `config.yaml`, or `config.json` in the current directory, or via the `--config` flag.

Example `config.toml`:

```toml
# Your RescueGroups.org API Key
api_key = "YOUR_API_KEY_HERE"

# Default search parameters (used if not provided by the agent)
postal_code = "90210"
miles = 50
species = "dogs"

# Lazy Loading (MCP Mode)
# If true, only a core set of tools is initially exposed to the client.
# Other tools can be discovered via 'inspect_tool'.
lazy = true

# Rate Limiting
# Protect your API key by limiting the number of requests per window.
# Default: 60 requests per 60 seconds (1 request per second)
rate_limit_requests = 60
rate_limit_window = 60
```

### :earth_africa: Environment Variables

You can also configure the server using environment variables:
- `RESCUE_GROUPS_API_KEY`: Rescue Groups [API Key][1].
- `MCP_AUTH_TOKEN`: Bearer token for authentication in HTTP mode.
- `RUST_LOG_FORMAT`: Set to `json` for structured logging.
- `RUST_LOG`: Control logging verbosity (e.g., `RUST_LOG=info,rescue_groups_mcp=debug`).

## :balance_scale: License

​[​Apache License 2.0](https://raw.githubusercontent.com/nicholaswilde/rescue-groups-mcp/refs/heads/main/LICENSE)

## :writing_hand: Author

​This project was started in 2026 by [Nicholas Wilde][2].

[1]: <https://userguide.rescuegroups.org/spaces/APIDG/pages/8192120/API+Developers+Guide+Home>
[2]: <https://github.com/nicholaswilde/>
[3]: <https://rescuegroups.org/>
