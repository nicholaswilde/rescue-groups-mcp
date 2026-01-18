# Specification: Live Integration Testing

## Goal
Verify that all read-only MCP tools function correctly against the live RescueGroups.org API using the provided API key.

## Scope
-   Execute CLI commands for each tool.
-   Verify output is non-empty and error-free.
-   Record results.

## Tools to Test
1.  `list_species`
2.  `list_breeds`
3.  `list_metadata`
4.  `search_adoptable_pets` (and `list_animals`)
5.  `get_animal_details`
6.  `get_contact_info`
7.  `compare_animals`
8.  `search_organizations`
9.  `get_organization_details`
10. `list_org_animals`
11. `list_adopted_animals`
12. `inspect_tool` (Internal, but good to verify)

## Method
-   Use `cargo run -- <command> <args>`
-   Use `config.toml` for authentication.
