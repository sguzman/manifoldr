# Manifoldr revival roadmap

Updated: 2026-10-15. Stewardship goal: maintain a safe, dependable Rust CLI for the public Manifold Markets API. Changes go directly to the default branch in coherent commits; trading remains opt-in.

## P0 — Stabilize and recover (completed 2026-10-15)

- [x] Inventory existing commands, source code, dependencies, and historical output
- [x] Recheck upstream API documentation and identify schema and accounting risks
- [x] Make trade execution explicitly opt-in; default to API-supported simulation
- [x] Stop estimating portfolio P/L from raw share counts; display API metrics
- [x] Narrow position/contract deserialization to the fields actually needed
- [x] Add bounded HTTP timeouts, transient GET retries, and actionable error output
- [x] Add consistent JSON output and safe watch polling behavior
- [x] Add regression fixtures, offline unit tests, rustfmt/clippy/test CI
- [x] Clean accidental generated artifacts from version control
- [x] Rewrite README and add development/security notes
- [x] Verify a clean build, unit tests, and representative live read-only API calls
  - 2026-10-15: GitHub Actions [Rust CI run 38796201375](https://github.com/sguzman/manifoldr/actions/runs/38796201375) passed rustfmt, Clippy, unit tests, CLI help and public market-list API smoke.
  - Authenticated portfolio commands and real order execution intentionally NOT exercised in CI.


## P1 — Expand endpoint coverage

2026-10-15 progress: [CI run 38801962820](https://github.com/sguzman/manifoldr/actions/runs/38801962820) passed strict formatting, Clippy, 20 offline tests, CLI help and three separate read-only public API smoke checks (markets, users, topics). Authenticated actions remain intentionally untested.

- [x] Split reusable API client into a library crate; CLI consumes it (further handler decomposition optional)
- [x] Public `ApiResult`/typed `ApiError` for API callers; explicit `user get --id` supported
- [ ] Provide explicit immutable-ID mode for portfolio/history/positions (currently username-first fallback)
- [ ] Complete pagination/filter coverage (market, bet and user cursors; position offset/order/per-answer; market creator/topic filters implemented). Portfolio history has no documented pagination cursor.
- [x] Single and bulk market probability retrieval with structured JSON output
- [x] Group/topic and user discovery endpoints (listing with cursors and topic by slug)
- [x] Order inspection/cancellation and share-selling flows (all read-only or local preview by default; real cancellations and sales require `--execute`; no real writes tested)
- [ ] Live REST compatibility fixtures; capture only non-sensitive public responses
- [x] Integration tests using a local HTTP mock server (never authenticated live trading)
- [ ] API limits, backoff, observability, and resilience for intermittent networks (429 is typed and not retried; further resilience work open)

## P2 — Usability and real-time functionality

- [ ] WebSocket streaming and connection re-establishment
- [ ] Local caching, incremental sync and offline history
- [ ] Reliable portfolio valuation for multi-outcome, mana/cash and loan cases
- [ ] Search ergonomics, shell completions and terminal formatting options
- [ ] Document stable machine-readable JSON schemas
- [ ] Release binaries for Linux, changelog, packaging and versioning policy

## Constraints and quality gates

- Linux-first, Rust-based, terminal-friendly, no paid dependencies.
- Never commit API keys, private trading data, personal logs or .env files.
- No unintended real trades: write operations require deliberate user opt-in.
- Never retry POST automatically (duplicate order risk).
- Keep requests within Manifold's published 500 requests/minute per-IP limit.
- Tests/CI must be safe to run without API credentials.
- Keep README command examples synchronized with `--help`.
- A roadmap checkbox means verified work, not a plan or an assumption.

References: https://docs.manifold.markets/api and https://github.com/manifoldmarkets/manifold/blob/main/docs/docs/api.md
