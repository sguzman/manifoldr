# Changelog

## 0.2.0 — release candidate (2026-10-15)

- Added public real-time WebSocket subscriptions with keepalive pings, resubscription, bounded backoff and newline-delimited JSON output.
- Added fish/bash/zsh and other shell completion generation.
- Added immutable user-ID options to portfolio, history and positions commands.
- Added contract type, creator ID, topic slug and offset filtering to market search.
- Recognize the API's `prob` market field and optional cash portfolio metrics.
- Added local WebSocket reconnection regression tests with no external network.
- Added Linux x86-64 GNU tarball build and checksums in a GitHub Actions workflow, with tagged-release publishing support.



## Unreleased — revival work (2026-10-15)

### Added (P1 underway)

- `market sell` safely previews a YES/NO share sale locally and requires `--execute` for real submission; `--shares` and `--all` are mutually exclusive.
- Offline mock-server tests cover explicit share-sale requests, omit shares for --all, reject invalid inputs and prohibit automatic POST retries.

- Read-only open-limit order inspection and expanded bet filters (username, market slug, timestamp, order).
- Local-only `bet cancel BET_ID` preview; real cancellation POST requires `--execute`, validates the ID and never retries.
- Optional `orderAmount` and `limitProb` bet fields and four new offline HTTP contract tests.

- Public `ApiResult<T>` with typed API error variants and HTTP status inspection, distinguishing throttling from decoding or network failures.
- Portfolio contract pagination (`--offset`, `--order`, `--per-answer`) and creator/topic filters for market listing.
- Direct immutable-ID user lookups with `user get --id`, preserving existing username-first fallback.
- Offline mock-server coverage for paginated position queries, market filters, input validation, and 429 responses.

- Reusable `manifoldr` Rust library; the existing binary now imports its public client and models.
- Fallible client constructor and configurable base URL for offline mock-server tests.
- Public user/topic discovery, individual market probability lookup, and pagination cursors for market search and bet list.
- Local HTTP contract tests for query encoding, transient GET retries, 404 errors, dry-run payloads, and never-retried POSTs.
- Topic `about` now supports Manifold's structured rich-text JSON instead of expecting a string; discovered by live smoke test.
- Public live smoke commands run as separate best-effort checks, so one failure cannot hide later probes.

### Changed

- **Breaking trading safety change:** `bet place` now submits Manifold's `dryRun: true` by default. Real trades require `--execute`.
- Page-specific position totals are now labeled `PAGE TOTAL` to avoid misleading account-wide summaries.
- Position tables display API-provided invested/profit/profit-percent metrics instead of treating share counts as a market payout valuation.
- Username lookup falls back to ID lookup only when the username endpoint returns HTTP 404.
- Added bounded 10-second connection / 30-second request timeouts, limited transient GET retries, and clearer HTTP errors; never retry POST.
- Added global `--json` on table-oriented commands.
- Added bulk `market probabilities` for up to 100 IDs.
- Prevented ultra-fast position watch loops, invalid bet amounts and unsupported outcomes.
- Portfolio contract responses deserialize into a narrower contract summary, independent of public market listing fields.
- Added regression fixtures and GitHub Actions Rust quality checks.
- Reorganized documentation and removed committed scratch output files.

### Validation

- [GitHub Actions run 38796201375](https://github.com/sguzman/manifoldr/actions/runs/38796201375) passed Rust formatting, Clippy, unit tests, CLI help, and a live *public* market-list API smoke on 2026-10-15.
- [P1 CI run 38801962820](https://github.com/sguzman/manifoldr/actions/runs/38801962820) passed strict formatting, Clippy, 20 offline Rust tests (including 13 local HTTP tests), CLI help, and live read-only market, user and topic requests. These probes are best-effort and remain separately labeled, not required for offline builds.
- Authenticated user operations and actual trading were not exercised in CI; additional endpoint coverage and integration cases remain tracked on the roadmap.

## 0.1.0 — initial codebase

- Async Manifold client, user/portfolio inspection, market listing/search, position tables and basic betting.
