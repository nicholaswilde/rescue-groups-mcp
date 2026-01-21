# Specification: New Features (Org Name, Metadata, Random Pet)

## Goal
Enhance the MCP server with high-value usability features: organization name search, metadata discovery, and a "surprise me" random pet function.

## Features

### 1. Organization Name Search
-   **Purpose:** Allow users to find rescue organizations by name (e.g., "Golden Retriever Rescue") instead of just location.
-   **Inputs:** Add `query` (string) to `search_organizations` tool.
-   **Logic:** Use the `orgs.name` field with `contains` operation in the API filter.

### 2. Metadata Type Discovery
-   **Purpose:** Allow users/agents to discover what valid `metadata_type` values exist for `list_metadata`.
-   **Tool Name:** `list_metadata_types`
-   **Output:** List of strings (e.g., "colors", "patterns", "qualities", "species", "breeds").

### 3. Random Pet
-   **Purpose:** A fun, zero-config way to see an adoptable animal.
-   **Tool Name:** `get_random_pet`
-   **Inputs:** None (or optional `species`).
-   **Logic:** Call `fetch_pets` with `sort_by="Random"` and `limit=1`.

## Implementation Details
-   Update `OrgSearchArgs` struct.
-   Update `search_organizations` function.
-   Implement `list_metadata_types` function (static list or API discovery if possible, likely static list of API endpoints).
-   Implement `get_random_pet` function.
-   Update tool definitions.
