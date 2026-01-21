# Specification: Automated Release Workflow

## Goal
Automate the distribution of the MCP server by creating GitHub Releases with cross-platform binaries and publishing Docker images upon pushing a version tag.

## Triggers
-   Pushing a tag matching `v*` (e.g., `v0.1.0`).

## Jobs

### 1. Build & Release Binaries
-   **Strategy:** Matrix build for key platforms.
    -   `x86_64-unknown-linux-gnu` (Ubuntu)
    -   `aarch64-unknown-linux-gnu` (ARM64 Linux)
    -   `x86_64-apple-darwin` (macOS Intel)
    -   `aarch64-apple-darwin` (macOS Silicon)
    -   `x86_64-pc-windows-msvc` (Windows)
-   **Actions:**
    -   Checkout code.
    -   Install Rust toolchain.
    -   Build with `cargo build --release`.
    -   Archive binary (tar.gz or zip).
    -   Upload to GitHub Release.

### 2. Docker Publish
-   **Registry:** GitHub Container Registry (ghcr.io).
-   **Platforms:** `linux/amd64`, `linux/arm64`.
-   **Tags:** `latest`, `vX.Y.Z` (semver).
-   **Actions:**
    -   Checkout code.
    -   Set up QEMU (for multi-arch).
    -   Set up Docker Buildx.
    -   Login to GHCR.
    -   Build and push.

## Deliverables
-   `.github/workflows/release.yml`
