# Implementation Plan: Implement Lazy Loading for Tools

## Phase 1: Analysis & Design [Done]

- [x] Task: Analyze `proxmox-mcp-rs` (Done via proxy/assumption)
- [x] Task: Define the categorization/grouping strategy (Core vs Hidden)

## Phase 2: Implementation

- [ ] Task: Refactor Tool Definitions
    - [ ] Create `tools.rs` or helper functions in `main.rs` to define all tool schemas.
    - [ ] Separate `core_tools` and `all_tools`.
- [ ] Task: Update Configuration
    - [ ] Add `lazy` field to `Settings` and `ConfigFile`.
- [ ] Task: Implement `inspect_tool`
    - [ ] Add logic to list all tools or show specific schema.
- [ ] Task: Update `tools/list` Handler
    - [ ] Return filtered list based on `lazy` setting.
- [ ] Task: Conductor - User Manual Verification 'Implementation' (Protocol in workflow.md)

## Phase 3: Verification

- [ ] Task: Verify `tools/list` output in lazy mode.
- [ ] Task: Verify `inspect_tool` output.
- [ ] Task: Verify hidden tools can still be called.
- [ ] Task: Conductor - User Manual Verification 'Verification' (Protocol in workflow.md)