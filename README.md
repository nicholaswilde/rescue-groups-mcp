# :dog: Rescue Groups MCP :robot:

[![task](https://img.shields.io/badge/Task-Enabled-brightgreen?style=for-the-badge&logo=task&logoColor=white)](https://taskfile.dev/#/)

> [!WARNING]
> This project is currently in active development (v0.x.x) and is **not production-ready**. Features may change, and breaking changes may occur without notice.

An MCP server to interface with [Rescue Groups][3] written in Rust.

You will need to request an [API key][1] from the group.

## :sparkles: Features

WIP

## :hammer_and_wrench: Build

To build the project, you need a Rust toolchain installed.

```bash
cargo build --release
```

The binary will be available at `target/release/rescue-groups-mcp`.

## :rocket: Usage

### :keyboard: Command Line Arguments

```bash
./target/rescue-groups-mcp --help
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
