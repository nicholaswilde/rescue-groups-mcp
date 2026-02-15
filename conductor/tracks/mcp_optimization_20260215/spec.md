# Specification: MCP Optimization & Polymorphic Tools

## Overview
This track aims to reduce the token footprint of the RescueGroups MCP server by consolidating granular tools into polymorphic functions and thinning response payloads. This will improve context window efficiency, reduce latency, and lower the cost of agent interactions.

## Functional Requirements

### 1. Tool Consolidation (Polymorphism)
Replace existing granular tools with four primary polymorphic tools:
- **`pet_tool`**: Unified interface for `search_animals`, `get_animal_details`, `get_random_animal`, `compare_animals`, and `success_stories`.
- **`org_tool`**: Unified interface for `search_orgs` and `get_org_details`.
- **`event_tool`**: Unified interface for `search_events` and `get_event_details`.
- **`metadata_tool`**: Unified interface for all metadata discovery (breeds, colors, patterns, etc.).

### 2. Response Thinning (Summary vs. Detail)
- Implement a `full_details: boolean` parameter (default: `false`) for all tools.
- When `false`, return a "Summary" object containing only essential fields (e.g., ID, Name, Species, Primary Breed).
- When `true`, return the complete RescueGroups API response.

### 3. Schema & Description Optimization
- **Description Trimming:** Shorten all tool and argument descriptions to the minimum necessary for agent comprehension.
- **Schema Simplification:** Use compact parameter names (e.g., `id` instead of `animal_id` or `org_id`).
- **Enums:** Use hardcoded enums for fixed categories (Species, Status, etc.) to replace long list-style descriptions.

### 4. Documentation & Test Alignment
- **Test Suite Update:** Refactor all existing unit and integration tests to verify the new polymorphic tool structure and response thinning logic.
- **README Update:** Update `README.md` to reflect the new tool definitions, parameters, and usage examples.

## Non-Functional Requirements
- **Token Reduction Target:** 
  - 20%+ reduction in `list_tools` token count.
  - 25%+ reduction in total tool count.
  - 15%+ reduction in average response token count.
- **Performance:** Ensure consolidation logic does not increase response latency.
- **Breaking Change:** This is a breaking change; existing granular tools will be removed.

## Acceptance Criteria
- [ ] New polymorphic tools are implemented and functional.
- [ ] `full_details` parameter correctly toggles between summary and full payloads.
- [ ] Total tool count is reduced by at least 25%.
- [ ] All existing tests pass using the new tools.
- [ ] `README.md` is updated with current tool documentation.
- [ ] A token count comparison (Before vs. After) confirms the reduction targets.

## Out of Scope
- Implementing write-access tools (adoption applications, etc.).
- Changes to the underlying caching or rate-limiting logic (unless required by new tool structure).
