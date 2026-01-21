# Implementation Plan: New Features

## Phase 1: Organization Search by Name
- [x] Task: Update `OrgSearchArgs` in `src/main.rs` to include `query: Option<String>`.
- [x] Task: Update `search_organizations` to handle the `query` filter.
- [x] Task: Update tool definition for `search_organizations`.
- [x] Task: Add test case for org name search.

## Phase 2: Metadata Types
- [x] Task: Implement `list_metadata_types` function (returning static list of valid types).
- [x] Task: Add `list_metadata_types` to tool definitions and `handle_tool_call`.
- [x] Task: Add CLI command `ListMetadataTypes`.
- [x] Task: Test metadata types list.

## Phase 3: Random Pet
- [x] Task: Implement `get_random_pet` function (wrapper around `fetch_pets` with random sort).
- [x] Task: Add `get_random_pet` to tool definitions.
- [x] Task: Add CLI command `RandomPet`.
- [x] Task: Test random pet function.

## Phase 4: Verification
- [x] Task: Verify all new tools with `cargo test`.
