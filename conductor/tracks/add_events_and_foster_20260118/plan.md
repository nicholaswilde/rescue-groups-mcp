# Implementation Plan: Events and Foster Support

## Phase 1: Foster Support

- [ ] Task: Add `needs_foster` to `ToolArgs` struct in `src/main.rs`
- [ ] Task: Update `fetch_pets` to handle `needs_foster` filter
- [ ] Task: Update tool definition for `search_adoptable_pets` in `get_all_tool_definitions`
- [ ] Task: Add unit test for foster search
- [ ] Task: Conductor - User Manual Verification 'Foster Support' (Protocol in workflow.md)

## Phase 2: Events Support

- [ ] Task: Define `EventSearchArgs` struct
- [ ] Task: Implement `search_events` function calling `/public/events/search`
- [ ] Task: Implement `format_event_results`
- [ ] Task: Add `list_events` to tool definitions and `handle_tool_call`
- [ ] Task: Add `list_events` to `Commands` enum (CLI)
- [ ] Task: Add unit test for events
- [ ] Task: Conductor - User Manual Verification 'Events Support' (Protocol in workflow.md)

## Phase 3: Finalization

- [ ] Task: Update README with new features
- [ ] Task: Conductor - User Manual Verification 'Finalization' (Protocol in workflow.md)
