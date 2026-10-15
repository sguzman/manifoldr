# Changelog

## Unreleased — revival work (2026-10-15)

### Changed

- **Breaking trading safety change:** `bet place` now submits Manifold's `dryRun: true` by default. Real trades require `--execute`.
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
- Authenticated user operations and actual trading were not exercised in CI; further API coverage and integration tests are tracked on the roadmap.

## 0.1.0 — initial codebase

- Async Manifold client, user/portfolio inspection, market listing/search, position tables and basic betting.
