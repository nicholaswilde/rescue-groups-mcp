# Implementation Plan: MCP Optimization & Polymorphic Tools

## Phase 1: Baseline & Preparation
- [ ] Task: Establish Token Baseline
    - [ ] Generate current `list_tools` output and record token count.
    - [ ] Record token counts for representative tool responses (search, details).
- [ ] Task: Setup Polymorphic Test Suite
    - [ ] Create `tests/polymorphic_tools.rs` to house tests for new tools.
    - [ ] Define shared mock responses for the new tool structures.
- [ ] Task: Conductor - User Manual Verification 'Baseline & Preparation' (Protocol in workflow.md)

## Phase 2: Core Tool Consolidation
- [ ] Task: Implement `pet_tool` (TDD)
    - [ ] Write failing tests for consolidated pet search, details, and random pet functionality.
    - [ ] Implement `pet_tool` logic in `src/mcp.rs` and `src/server.rs`.
    - [ ] Verify tests pass and consolidate logic.
- [ ] Task: Implement `org_tool` (TDD)
    - [ ] Write failing tests for consolidated organization search and details.
    - [ ] Implement `org_tool` logic.
    - [ ] Verify tests pass.
- [ ] Task: Implement `event_tool` (TDD)
    - [ ] Write failing tests for consolidated event search and details.
    - [ ] Implement `event_tool` logic.
    - [ ] Verify tests pass.
- [ ] Task: Implement `metadata_tool` (TDD)
    - [ ] Write failing tests for unified metadata retrieval.
    - [ ] Implement `metadata_tool` logic.
    - [ ] Verify tests pass.
- [ ] Task: Conductor - User Manual Verification 'Core Tool Consolidation' (Protocol in workflow.md)

## Phase 3: Response Thinning & Schema Optimization
- [ ] Task: Implement Summary Logic (TDD)
    - [ ] Write tests for `full_details: false` (expecting minimal payload).
    - [ ] Implement Summary structs and mapping logic for all entities.
    - [ ] Verify tests pass.
- [ ] Task: Apply Schema & Description Trimming
    - [ ] Shorten tool and argument descriptions in MCP definitions.
    - [ ] Rename parameters to compact forms (e.g., `id`).
    - [ ] Implement Enums for fixed API fields.
- [ ] Task: Conductor - User Manual Verification 'Response Thinning & Schema Optimization' (Protocol in workflow.md)

## Phase 4: Clean-up & Verification
- [ ] Task: Remove Legacy Tools
    - [ ] Delete original granular tool definitions and associated logic.
    - [ ] Clean up redundant code in `src/commands.rs`.
- [ ] Task: Final Documentation & Documentation
    - [ ] Update `README.md` with new tool usage and examples.
    - [ ] Update any in-code documentation and comments.
- [ ] Task: Final Token Audit
    - [ ] Generate new `list_tools` output and verify >20% reduction.
    - [ ] Verify average response size reduction targets.
- [ ] Task: Conductor - User Manual Verification 'Clean-up & Verification' (Protocol in workflow.md)
