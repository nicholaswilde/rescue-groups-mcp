# Specification: Events and Foster Support

## Goal
Expand the MCP server capabilities to include adoption events and foster opportunities, increasing the chances of animals finding homes.

## Features

### 1. `list_events` Tool
-   **Purpose:** Allow users to find adoption events near them.
-   **Inputs:**
    -   `postal_code` (optional, defaults to config)
    -   `miles` (optional, defaults to config)
-   **Output:** List of events with Name, Date, Location, and Organization.
-   **API Endpoint:** `/public/events/search` (POST).

### 2. Foster Filter in `search_adoptable_pets`
-   **Purpose:** specific search for animals that need fostering.
-   **Inputs:** Add `needs_foster` (boolean) to existing `search_adoptable_pets` inputs.
-   **Logic:** Filter by `animals.isNeedingFoster` (or equivalent API field).

## Implementation Details
-   **Events:** Requires a new struct `EventSearchArgs`, new function `search_events`, and update to tool definitions.
-   **Foster:** Update `ToolArgs` struct and `fetch_pets` filter construction.
