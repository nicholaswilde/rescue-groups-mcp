# Implementation Plan: API Rate Limiting

## Phase 1: Setup & Configuration
- [x] Task: Add `governor` and `nonzero_ext` to `Cargo.toml`.
- [ ] Task: Update `ConfigFile` struct in `src/main.rs` to include `rate_limit_requests` and `rate_limit_window`.
- [ ] Task: Update `Settings` struct to hold the `Quota` or `RateLimiter` instance (wrapped in `Arc`).

## Phase 2: Implementation
- [x] Task: Initialize the `RateLimiter` in `merge_configuration`.
- [x] Task: Update `fetch_with_cache` to check the limiter before `reqwest` calls.
- [x] Task: Implement graceful waiting or error return on limit violation.

## Phase 3: Verification
- [x] Task: Add a unit test `test_rate_limiting` using a mock clock or small capacity (e.g., 2 req/sec) to verify blocking/error behavior.
- [x] Task: Verify that cache hits do *not* consume rate limit tokens. (Implicit in implementation: limiter check is after cache check)
