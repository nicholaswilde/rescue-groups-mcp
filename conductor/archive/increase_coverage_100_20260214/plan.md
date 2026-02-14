# Implementation Plan: Increase Code Coverage to 100%

## Phase 1: Core Utilities & Configuration [checkpoint: 1366fea]

- [x] Task: Close gaps in `config.rs` 1366fea
    - [x] Write tests for TOML/JSON/YAML file parsing success/failure
    - [x] Test all default value fallbacks in `merge_configuration`
- [x] Task: Complete coverage for `error.rs` 1366fea
    - [x] Test all `Display` implementations for `AppError`
    - [x] Test all `From` conversions (Io, Toml, Reqwest, etc.)
- [x] Task: Minor gaps in `cli.rs` and `fmt.rs` 1366fea
    - [x] Identify and cover remaining lines in `cli.rs` (likely parser edge cases)
    - [x] Cover remaining branches in `fmt.rs`
- [x] Task: Conductor - User Manual Verification 'Core Utilities & Configuration' (Protocol in workflow.md)

## Phase 2: Protocol & Client Logic [checkpoint: 1366fea]

- [x] Task: Exhaustive testing for `mcp.rs` 1366fea
    - [x] Test notification handlers (initialized)
    - [x] Test tool inspection logic branches (found vs not found)
    - [x] Test JSON-RPC error formatting edge cases
- [x] Task: Polish `client.rs` and `commands.rs` 1366fea
    - [x] Test rate limiter wait logic (if possible via mock clock or timing)
    - [x] Test `handle_command` for `Generate` (Man page path)
    - [x] Identify and cover remaining lines in client API calls
- [x] Task: Conductor - User Manual Verification 'Protocol & Client Logic' (Protocol in workflow.md)

## Phase 3: Infrastructure & Integration [checkpoint: 1366fea]

- [x] Task: Increase coverage for `server.rs` 1366fea
    - [x] Test SSE session timeout/disconnection behavior
    - [x] Test HTTP auth failure scenarios in more detail
    - [x] Test JSON parsing errors in HTTP handlers
- [x] Task: Cover `main.rs` logic 1366fea
    - [x] Refactor `main` logic into testable sub-functions if necessary
    - [x] Add integration-style tests that exercise the CLI-to-server dispatching logic
- [x] Task: Identify and Apply Strategic Exclusions 1366fea
    - [x] Audit code for truly unreachable paths (e.g., certain IO panics)
    - [x] Apply `#[cfg(not(test))]` or similar to exclude them
- [x] Task: Conductor - User Manual Verification 'Infrastructure & Integration' (Protocol in workflow.md)

## Phase 4: Threshold Enforcement & Finalization [checkpoint: 1366fea]

- [x] Task: Update Taskfile Enforcement 1366fea
    - [x] Change `fail-under-lines` from 90 to 98 in `Taskfile.yml`
- [x] Task: Final Coverage Audit 1366fea
    - [x] Run `task coverage` and ensure >= 98% total
- [x] Task: Update README documentation 1366fea
    - [x] Update coverage badge and goal section in `README.md`
- [x] Task: Conductor - User Manual Verification 'Threshold Enforcement & Finalization' (Protocol in workflow.md)
