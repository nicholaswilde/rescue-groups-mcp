# :dog: RescueGroups MCP Server :robot:

[![task](https://img.shields.io/badge/Task-Enabled-brightgreen?style=for-the-badge&logo=task&logoColor=white)](https://taskfile.dev/#/)

> [!WARNING]
> This project is currently in active development (v0.x.x) and is **not production-ready**. Features may change, and breaking changes may occur without notice.

An MCP server to interface with [RescueGroups][3] written in Rust.

You will need to request an [API key][1] from the group.

## :sparkles: Features

- **MCP Integration**: Fully compatible with the Model Context Protocol for use with LLMs like Claude.
- **Advanced Caching**: Built-in asynchronous caching (15-minute TTL) using `moka` to reduce API load and stay within rate limits.
- **Multiple Config Formats**: Support for TOML, YAML, and JSON configuration files.
- **Rich Results**: Returns Markdown-formatted animal profiles with embedded images and detailed descriptions.
- **Robustness**: Comprehensive unit and integration test suite with mocked API responses.

## :toolbox: MCP Tools

- `search_adoptable_pets`: Find pets near you by species, postal code, and radius.
    - **Filters**: `good_with_children`, `good_with_dogs`, `good_with_cats`, `house_trained`, `special_needs`.
    - **Sorting**: Sort by `Newest`, `Distance`, or `Random`.
- `list_animals`: Browse the most recent adoptable animals available globally.
- `list_adopted_animals`: List recently adopted animals (Success Stories) to see happy endings near you.
- `get_animal_details`: Fetch a complete profile for a specific animal (description, sex, age, size, and photos).
- `list_species`: List all animal species supported by the API (e.g., Dog, Cat, Horse).
- `list_metadata`: List valid metadata values for animal attributes (colors, patterns, qualities).
- `list_breeds`: Discover available breeds for a specific species to refine your searches.
- `search_organizations`: Find animal rescue organizations and shelters by location.
- `get_organization_details`: Fetch a complete profile for a specific organization (mission, address, and contact info).
- `list_org_animals`: List all animals available for adoption at a specific shelter.

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

# Get raw JSON output (useful for scripting with jq)
./target/release/rescue-groups-mcp search --species cats --json | jq .

# List available species
./target/release/rescue-groups-mcp list-species

# Start the MCP server (default behavior)
./target/release/rescue-groups-mcp server

# Start the MCP server in HTTP mode
./target/release/rescue-groups-mcp http --port 8080 --auth-token mysecrettoken
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

The server can load configuration from a file named `config.toml`, `config.yaml`, or `config.json` in the current directory, or via the `--config` flag. See `config.toml.example` for details.

### :earth_africa: Environment Variables

You can also configure the server using environment variables:
- `RESCUE_GROUPS_API_KEY`: Rescue Groups [API Key][1].

## :balance_scale: License

​[​Apache License 2.0](https://raw.githubusercontent.com/nicholaswilde/rescue-groups-mcp/refs/heads/main/LICENSE)

## :writing_hand: Author

​This project was started in 2026 by [Nicholas Wilde][2].

[1]: <https://userguide.rescuegroups.org/spaces/APIDG/pages/8192120/API+Developers+Guide+Home>
[2]: <https://github.com/nicholaswilde/>
[3]: <https://rescuegroups.org/>
