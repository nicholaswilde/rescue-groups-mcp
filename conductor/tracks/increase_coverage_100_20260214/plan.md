# Implementation Plan: Increase Code Coverage to 100%

## Phase 1: Core Utilities & Configuration

- [ ] Task: Close gaps in `config.rs`
    - [ ] Write tests for TOML/JSON/YAML file parsing success/failure
    - [ ] Test all default value fallbacks in `merge_configuration`
- [ ] Task: Complete coverage for `error.rs`
    - [ ] Test all `Display` implementations for `AppError`
    - [ ] Test all `From` conversions (Io, Toml, Reqwest, etc.)
- [ ] Task: Minor gaps in `cli.rs` and `fmt.rs`
    - [ ] Identify and cover remaining lines in `cli.rs` (likely parser edge cases)
    - [ ] Cover remaining branches in `fmt.rs`
- [ ] Task: Conductor - User Manual Verification 'Core Utilities & Configuration' (Protocol in workflow.md)

## Phase 2: Protocol & Client Logic

- [ ] Task: Exhaustive testing for `mcp.rs`
    - [ ] Test notification handlers (initialized)
    - [ ] Test tool inspection logic branches (found vs not found)
    - [ ] Test JSON-RPC error formatting edge cases
- [ ] Task: Polish `client.rs` and `commands.rs`
    - [ ] Test rate limiter wait logic (if possible via mock clock or timing)
    - [ ] Test `handle_command` for `Generate` (Man page path)
    - [ ] Identify and cover remaining lines in client API calls
- [ ] Task: Conductor - User Manual Verification 'Protocol & Client Logic' (Protocol in workflow.md)

## Phase 3: Infrastructure & Integration

- [ ] Task: Increase coverage for `server.rs`
    - [ ] Test SSE session timeout/disconnection behavior
    - [ ] Test HTTP auth failure scenarios in more detail
    - [ ] Test JSON parsing errors in HTTP handlers
- [ ] Task: Cover `main.rs` logic
    - [ ] Refactor `main` logic into testable sub-functions if necessary
    - [ ] Add integration-style tests that exercise the CLI-to-server dispatching logic
- [ ] Task: Identify and Apply Strategic Exclusions
    - [ ] Audit code for truly unreachable paths (e.g., certain IO panics)
    - [ ] Apply `#[cfg(not(test))]` or similar to exclude them
- [ ] Task: Conductor - User Manual Verification 'Infrastructure & Integration' (Protocol in workflow.md)

## Phase 4: Threshold Enforcement & Finalization

- [ ] Task: Update Taskfile Enforcement
    - [ ] Change `fail-under-lines` from 90 to 98 in `Taskfile.yml`
- [ ] Task: Final Coverage Audit
    - [ ] Run `task coverage` and ensure >= 98% total
- [ ] Task: Update README documentation
    - [ ] Update coverage badge and goal section in `README.md`
- [ ] Task: Conductor - User Manual Verification 'Threshold Enforcement & Finalization' (Protocol in workflow.md)
