# Implementation Plan: Live Integration Testing

## Phase 1: Reference Data Gathering

- [ ] Task: Execute `list_species` to confirm basic connectivity and get a species for subsequent tests
- [ ] Task: Execute `search_adoptable_pets` to get an `animal_id` for details tests
- [ ] Task: Execute `search_organizations` to get an `org_id` for org tests

## Phase 2: Tool Verification

- [ ] Task: Test `list_breeds` (using species from Phase 1)
- [ ] Task: Test `list_metadata`
- [ ] Task: Test `get_animal_details` (using ID from Phase 1)
- [ ] Task: Test `get_contact_info` (using ID from Phase 1)
- [ ] Task: Test `compare_animals` (using ID from Phase 1)
- [ ] Task: Test `get_organization_details` (using ID from Phase 1)
- [ ] Task: Test `list_org_animals` (using ID from Phase 1)
- [ ] Task: Test `list_adopted_animals`
- [ ] Task: Test `inspect_tool`

## Phase 3: Reporting

- [ ] Task: Summarize results and update `AGENTS.md` with verified status.
- [ ] Task: Conductor - User Manual Verification 'Live Testing' (Protocol in workflow.md)
