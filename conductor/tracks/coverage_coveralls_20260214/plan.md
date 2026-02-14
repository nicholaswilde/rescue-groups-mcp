# Implementation Plan: Manual Code Coverage with Coveralls

## Phase 1: Local Tooling & Configuration [checkpoint: 1366fea]

- [x] Task: Install and Configure `cargo-llvm-cov`
    - [x] Install `cargo-llvm-cov` locally (`cargo install cargo-llvm-cov`)
    - [x] Verify `cargo llvm-cov` runs and generates a basic terminal report
- [x] Task: Set up Coveralls Authentication
    - [x] Identify the correct CLI upload tool: `/usr/local/bin/coveralls`
    - [x] Create `.env.example` with `COVERALLS_REPO_TOKEN` placeholder
- [x] Task: Conductor - User Manual Verification 'Local Tooling & Configuration' (Protocol in workflow.md)

## Phase 2: Automation with Taskfile [checkpoint: 1366fea]

- [x] Task: Implement Local Coverage Task
    - [x] Add `test:coverage` task to `Taskfile.yml` to generate HTML reports
    - [x] Verify `task test:coverage` opens/generates a valid HTML report
- [x] Task: Implement Coveralls Upload Task
    - [x] Add `coverage:upload` task to `Taskfile.yml` to generate LCOV and upload
    - [x] Verify `task coverage:upload` correctly formats the LCOV file for Coveralls
- [x] Task: Conductor - User Manual Verification 'Automation with Taskfile' (Protocol in workflow.md)

## Phase 3: Increase Test Coverage to >90% [checkpoint: 1366fea]

- [x] Task: Coverage for `client.rs` 1366fea
    - [x] Add tests for `search_adoptable_pets`
    - [x] Add tests for `get_animal_details`
    - [x] Add tests for `get_contact_info`
    - [x] Add tests for `search_organizations`
    - [x] Add tests for `get_organization_details`
    - [x] Add tests for `search_events`
- [x] Task: Coverage for `fmt.rs` 1366fea
    - [x] Add tests for all formatting functions
- [x] Task: Coverage for `commands.rs` 1366fea
    - [x] Add tests for all CLI command handlers
- [x] Task: Coverage for `mcp.rs` 1366fea
    - [x] Add tests for all MCP protocol handlers
- [x] Task: Coverage for `server.rs` 1366fea
    - [x] Add tests for Axum routes and state management
- [x] Task: Coverage for `main.rs` 1366fea
    - [x] Add tests for `merge_configuration` and main entry point logic
- [x] Task: Conductor - User Manual Verification 'Increase Test Coverage' (Protocol in workflow.md)

## Phase 4: Documentation & Branding [checkpoint: 1366fea]

- [x] Task: Add Coveralls Badge
    - [x] Update `README.md` with the Coveralls status badge markdown
- [x] Task: Document Manual Coverage Process
    - [x] Add a "Code Coverage" section to `README.md` or `DEVELOPMENT.md` explaining prerequisites and the upload command
- [x] Task: Conductor - User Manual Verification 'Documentation & Finalization' (Protocol in workflow.md)
