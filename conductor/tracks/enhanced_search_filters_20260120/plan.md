# Plan: Enhanced Search Filters

## Goal
Expand the search capabilities of the `search_adoptable_pets` tool (and CLI `search` command) to support more specific attributes like color and pattern.

## Tasks

### 1. Define New Filters
- [x] Identify which fields to add.
    - `color` (string) -> `animals.colorDetails` contains.
    - `pattern` (string) -> `animals.patternDetails` contains.
    - (Dropped) `not_good_with_cats` etc. - API returns 400 Bad Request when filtering these fields with "equal". Requires further investigation into correct API usage (likely `qualities` filter).

### 2. Update CLI
- [x] Modify `src/cli.rs` `ToolArgs` struct to include `color` and `pattern`.

### 3. Update Client Logic
- [x] Modify `src/client.rs` `fetch_pets` function.
- [x] Use `add_filter` to map `color` and `pattern`.

### 4. Verification
- [x] Run `cargo check`.
- [x] Run binary with new flags: `cargo run -- --config config.toml search --color Black`.
- [x] Verify output restricts results accordingly. (Verified: Returns pets with "Black" in color/description).

## Outcome
Implemented `color` and `pattern` filtering. Exclusion filters were attempted but reverted due to API limitations/errors.