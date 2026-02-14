# Implementation Plan: Manual Code Coverage with Coveralls

## Phase 1: Local Tooling & Configuration

- [ ] Task: Install and Configure `cargo-llvm-cov`
    - [ ] Install `cargo-llvm-cov` locally (`cargo install cargo-llvm-cov`)
    - [ ] Verify `cargo llvm-cov` runs and generates a basic terminal report
- [ ] Task: Set up Coveralls Authentication
    - [ ] Identify the correct CLI upload tool (e.g., `coveralls-lcov`)
    - [ ] Create `.env.example` with `COVERALLS_REPO_TOKEN` placeholder
- [ ] Task: Conductor - User Manual Verification 'Local Tooling & Configuration' (Protocol in workflow.md)

## Phase 2: Automation with Taskfile

- [ ] Task: Implement Local Coverage Task
    - [ ] Add `test:coverage` task to `Taskfile.yml` to generate HTML reports
    - [ ] Verify `task test:coverage` opens/generates a valid HTML report
- [ ] Task: Implement Coveralls Upload Task
    - [ ] Add `coverage:upload` task to `Taskfile.yml` to generate LCOV and upload
    - [ ] Verify `task coverage:upload` correctly formats the LCOV file for Coveralls
- [ ] Task: Conductor - User Manual Verification 'Automation with Taskfile' (Protocol in workflow.md)

## Phase 3: Documentation & Branding

- [ ] Task: Add Coveralls Badge
    - [ ] Update `README.md` with the Coveralls status badge markdown
- [ ] Task: Document Manual Coverage Process
    - [ ] Add a "Code Coverage" section to `README.md` or `DEVELOPMENT.md` explaining prerequisites and the upload command
- [ ] Task: Conductor - User Manual Verification 'Documentation & Finalization' (Protocol in workflow.md)
