# Specification: API Rate Limiting

## Goal
Protect the RescueGroups.org API key from being banned by strictly enforcing rate limits on the client side using a Token Bucket algorithm.

## Requirements
1.  **Configuration:**
    *   Allow users to define `max_requests` (capacity) and `window_seconds` (replenishment period) in `config.toml`.
    *   Default safe values (e.g., 60 requests per minute).
2.  **Implementation:**
    *   Use the `governor` crate for robust, thread-safe, async rate limiting.
    *   Integrate the limiter into the `fetch_with_cache` function *before* making the network request (but *after* checking the cache to allow cached bursts).
3.  **Behavior:**
    *   If the limit is exceeded, wait (throttle) if the wait time is short (e.g., < 1 sec).
    *   If the wait time is long, reject the request immediately with a specific error (e.g., `AppError::RateLimitExceeded`).

## Dependencies
-   `governor`
-   `nonzero_ext` (helper for non-zero integers required by governor)

## Metrics
-   API Key safety (zero 429s from upstream).
