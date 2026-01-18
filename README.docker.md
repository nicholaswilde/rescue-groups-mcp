# RescueGroups MCP Server Docker Image

This is the Docker image for the **RescueGroups Model Context Protocol (MCP) Server**. It allows AI assistants (like Claude) to search for adoptable pets, list breeds, and find rescue organizations using the RescueGroups.org API.

## Prerequisites

You need a **RescueGroups API Key** to use this server.
1.  Register at [RescueGroups.org](https://www.rescuegroups.org/).
2.  Request an API key from their developer portal.

## Usage

### 1. Stdio Mode (Default)

This mode is designed for MCP clients that communicate via Standard Input/Output (like Claude Desktop).

```bash
docker run -i --rm \
  -e RESCUE_GROUPS_API_KEY=your_api_key_here \
  nicholaswilde/rescue-groups-mcp
```

### 2. HTTP Mode (SSE)

You can also run the server in HTTP mode, which exposes a server supporting Server-Sent Events (SSE).

```bash
docker run --rm -p 3000:3000 \
  -e RESCUE_GROUPS_API_KEY=your_api_key_here \
  nicholaswilde/rescue-groups-mcp \
  http
```

-   **Port**: Defaults to `3000`.
-   **Host**: Defaults to `0.0.0.0`.

## Configuration (Environment Variables)

| Variable | Description | Required | Default |
| :--- | :--- | :--- | :--- |
| `RESCUE_GROUPS_API_KEY` | Your RescueGroups.org API Key. | **Yes** | - |
| `MCP_AUTH_TOKEN` | Bearer token for HTTP authentication. | No | - |
| `RUST_LOG` | Log level (e.g., `debug`, `info`). | No | `error` |

## Integration with Claude Desktop

To use this with Claude Desktop, add the following to your `claude_desktop_config.json`:

```json
{
  "mcpServers": {
    "rescue-groups": {
      "command": "docker",
      "args": [
        "run",
        "-i",
        "--rm",
        "-e",
        "RESCUE_GROUPS_API_KEY=your_actual_api_key",
        "nicholaswilde/rescue-groups-mcp"
      ]
    }
  }
}
```

## Supported Architectures

-   `linux/amd64`
-   `linux/arm64`
-   `linux/arm/v7`
