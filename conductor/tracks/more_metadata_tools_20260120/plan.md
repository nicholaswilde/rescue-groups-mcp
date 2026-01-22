# Plan: More Metadata Tools

## Goal
Expand the metadata discovery capabilities of the server to allow more granular querying of animal attributes, specifically colors and breeds.

## Tasks

### 1. Investigation
- [x] Verify if `colors` metadata can be filtered by `species`.
    - Verified: hit `/public/animals/species/{id}/colors` successfully.
- [x] Verify `breeds` detail endpoint.
    - Verified: hit `/public/animals/breeds/{id}` successfully.

### 2. Implementation
- [x] **Tool: `list_metadata` (Enhanced)**
    - Modified `list_metadata` to accept an optional `species` argument.
    - If `species` is provided, filters metadata by species using species-specific endpoint.
- [x] **Tool: `get_breed`**
    - New tool to fetch details for a specific breed ID.
    - Inputs: `breed_id`.
    - Outputs: Name.

### 3. Updates
- [x] Update `src/cli.rs` (added `GetBreed`, updated `MetadataArgs`).
- [x] Update `src/client.rs` (added `get_breed_details`, added `resolve_species_id` helper).
- [x] Update `src/mcp.rs` (added `get_breed` tool, updated `list_metadata` tool).
- [x] Update `src/commands.rs` (added `GetBreed` handler).
- [x] Update `src/fmt.rs` (added `format_breed_details`).

### 4. Verification
- [x] Run `cargo check`.
- [x] Run binary to test new tools.
    - `list-metadata --species cats --metadata-type colors`: Success.
    - `get-breed --breed-id 1`: Success.

## Technical Details
- Used species-specific endpoints like `/public/animals/species/{species_id}/colors`.
- Handled species name-to-ID resolution using the existing helper logic (extracted to `resolve_species_id`).

## Dependencies
- None.