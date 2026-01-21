# Implementation Plan: Events and Foster Support

## Phase 1: Foster Support

- [x] Task: Add `needs_foster` to `ToolArgs` struct in `src/main.rs`
- [x] Task: Update `fetch_pets` to handle `needs_foster` filter
- [x] Task: Update tool definition for `search_adoptable_pets` in `get_all_tool_definitions`
- [x] Task: Add unit test for foster search
- [x] Task: Conductor - User Manual Verification 'Foster Support' (Protocol in workflow.md)

## Phase 2: Events Support

- [x] Task: Define `EventSearchArgs` struct
- [x] Task: Implement `search_events` function calling `/public/events/search`
- [x] Task: Implement `format_event_results`
- [x] Task: Add `list_events` to tool definitions and `handle_tool_call`
- [x] Task: Add `list_events` to `Commands` enum (CLI)
- [x] Task: Add unit test for events
- [x] Task: Conductor - User Manual Verification 'Events Support' (Protocol in workflow.md)

## Phase 3: Finalization

- [x] Task: Update README with new features
- [x] Task: Conductor - User Manual Verification 'Finalization' (Protocol in workflow.md)
