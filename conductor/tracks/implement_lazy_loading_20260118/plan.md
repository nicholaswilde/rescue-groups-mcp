# Implementation Plan: Implement Lazy Loading for Tools

## Phase 1: Analysis & Design [Done]

- [x] Task: Analyze `proxmox-mcp-rs` (Done via proxy/assumption)
- [x] Task: Define the categorization/grouping strategy (Core vs Hidden)

## Phase 2: Implementation [Done]

- [x] Task: Refactor Tool Definitions
    - [x] Create `tools.rs` or helper functions in `main.rs` to define all tool schemas.
    - [x] Separate `core_tools` and `all_tools`.
- [x] Task: Update Configuration
    - [x] Add `lazy` field to `Settings` and `ConfigFile`.
- [x] Task: Implement `inspect_tool`
    - [x] Add logic to list all tools or show specific schema.
- [x] Task: Update `tools/list` Handler
    - [x] Return filtered list based on `lazy` setting.
- [x] Task: Conductor - User Manual Verification 'Implementation' (Protocol in workflow.md)

## Phase 3: Verification [Done]

- [x] Task: Verify `tools/list` output in lazy mode.
- [x] Task: Verify `inspect_tool` output.
- [x] Task: Verify hidden tools can still be called.
- [x] Task: Conductor - User Manual Verification 'Verification' (Protocol in workflow.md)