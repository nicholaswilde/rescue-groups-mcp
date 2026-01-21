# Implementation Plan: Automated Release Workflow

## Phase 1: Workflow Definition
- [x] Task: Create `.github/workflows/release.yml`.
- [x] Task: Define the `release` job with matrix strategy for Linux, macOS, and Windows.
- [x] Task: Add steps to compress binaries (tar.gz/zip) with appropriate naming (e.g., `rescue-groups-mcp-linux-amd64.tar.gz`).
- [x] Task: Use `softprops/action-gh-release` to create the release and upload assets.

## Phase 2: Docker Integration
- [x] Task: Define the `docker` job in `release.yml`.
- [x] Task: Configure `docker/metadata-action` for tagging.
- [x] Task: Configure `docker/build-push-action` for multi-arch build and push.

## Phase 3: Verification (Manual)
- [x] Task: Commit the workflow file.
- [x] Task: Review the YAML syntax (can use a linter if available, or careful manual review).
- [ ] Task: (Optional) Push a test tag `v0.0.0-test` to verify trigger (requires user permission, likely skipped in this session).
