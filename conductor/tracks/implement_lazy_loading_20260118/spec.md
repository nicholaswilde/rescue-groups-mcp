# Specification: Implement Lazy Loading for Tools

## Goal
Reduce the token usage of the MCP server's initial `tools/list` response by hiding non-essential tools behind a discovery mechanism.

## Context
The server currently exposes 12+ tools. Sending full JSON schemas for all of them consumes significant context. By defaulting to a "Lazy Mode" where only core tools are listed, we save tokens.

## Requirements
1.  **Lazy Mode Flag:** Add `lazy_mode` boolean to `Settings` (default `true` or configurable).
2.  **Core Tools:** Always expose:
    - `search_adoptable_pets`
    - `get_animal_details`
    - `inspect_tool` (New discovery tool)
3.  **Discovery Tool (`inspect_tool`):**
    -   Input: `tool_name` (optional).
    -   Output:
        -   If `tool_name` is empty: List names and descriptions of ALL available tools.
        -   If `tool_name` is provided: Return the JSON schema for that tool.
4.  **Tool Execution:** Ensure `tools/call` handles ALL tools, even if they are not in the currently "listed" set (assuming the LLM will generate the call based on the schema it retrieved via `inspect_tool`).

## Design
-   **Configuration:** Add `lazy: bool` to `config.toml` and `Settings`.
-   **Structure:** Define all tools in a central registry or static list (currently they are hardcoded in `process_mcp_request`).
-   **Refactor:** Extract tool definitions into a helper function `get_tool_definitions()`.
-   **Logic:**
    -   `tools/list`:
        -   If `lazy=false`: Return `get_tool_definitions()`.
        -   If `lazy=true`: Return `get_core_definitions()` + `inspect_tool` definition.
-   **Tool:** `inspect_tool`:
    -   Lookup tool in `get_tool_definitions()`.
    -   Return description/schema as text/JSON.

## Caveats
-   This relies on the Client/LLM being able to call a tool that wasn't in the initial `tools/list` (Zero-shot tool use after discovery). Most agents support this if they know the schema.