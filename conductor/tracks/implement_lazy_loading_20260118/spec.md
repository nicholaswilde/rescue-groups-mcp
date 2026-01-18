# Specification: Implement Lazy Loading for Tools

## Goal
Reduce the token usage of the MCP server's initial `tools/list` response by implementing a "lazy loading" or hierarchical tool discovery mechanism, modeled after `proxmox-mcp-rs`.

## Context
Large tool lists consume significant context window tokens. By hiding specific tools behind a discovery mechanism (e.g., categories or a `help` tool), we can keep the initial prompt small and only load tool definitions when needed.

## Reference
- https://github.com/nicholaswilde/proxmox-mcp-rs

## Requirements
1.  **Analyze Reference:** Determine how `proxmox-mcp-rs` implements this.
2.  **Refactor Tool Listing:** Modify `tools/list` to return a reduced set of tools (or a meta-tool).
3.  **Implement Discovery:** Create a mechanism (e.g., `list_tools` with category, or `get_tool_definition`) to allow the LLM to discover other tools.
4.  **Preserve Functionality:** Ensure all existing functionalities (search, list, get) remain accessible.

## Design (Tentative - subject to analysis)
-   Maybe categorize tools: `Search`, `Details`, `Metadata`.
-   Initial list: `list_categories`, `get_tools_in_category`?
-   Or just keep core tools and hide niche ones?

Let's verify the reference first.
