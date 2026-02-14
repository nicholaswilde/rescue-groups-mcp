# Specification: Code Coverage with Coveralls (Manual)

## Overview
Implement the infrastructure for manual code coverage tracking and reporting for the RescueGroups MCP Server project using `cargo-llvm-cov`. This track focuses on setting up the local tools and commands necessary to accurately measure coverage and upload the results to `Coveralls` for visualization and historical tracking.

## Functional Requirements
- **Coverage Measurement:** Use `cargo-llvm-cov` to generate accurate code coverage reports locally.
- **Accuracy Configuration:** Configure `llvm-cov` flags to ensure precise reporting of executed lines and branches.
- **Manual Upload:** 
    - Identify and configure a CLI tool (e.g., `coveralls-lcov` or a similar helper) to upload coverage reports from the local machine to Coveralls.
    - Ensure the process handles the necessary authentication (via local environment variables).
- **Coverage Threshold:** The reporting tool or local command should provide a clear indication if the total code coverage is below 90%.
- **Task Automation:** Provide a simple way (e.g., a `Taskfile.yml` task) to run the entire flow: clean, measure, and upload.
- **Visual Status:** Add a Coveralls status badge to the project `README.md`.

## Non-Functional Requirements
- **Simplicity:** The manual upload process should be straightforward and require minimal configuration after the initial setup.
- **Security:** Ensure the Coveralls token is managed via local environment variables or a `.env` file (not committed to the repository).

## Acceptance Criteria
- [ ] `cargo-llvm-cov` is configured and runs successfully on the local developer machine.
- [ ] A local task (e.g., `task coverage:upload`) is functional and successfully uploads LCOV data to Coveralls.
- [ ] The Coveralls dashboard reflects the manually uploaded data.
- [ ] A Coveralls badge is visible in the `README.md` and reflects the project's coverage status.
- [ ] Documentation is provided on how to perform the manual coverage run and upload.

## Out of Scope
- **CI/CD Automation:** Automated coverage reporting via GitHub Actions or any other CI provider.
- Improving existing test coverage to reach the 90% mark.
