# Manifoldr revival roadmap

Updated: 2026-10-15. Stewardship goal: maintain a safe, dependable Rust CLI for the public Manifold Markets API. Changes go directly to the default branch in coherent commits; trading remains opt-in.

## P0 — Stabilize and recover (in progress)

- [x] Inventory existing commands, source code, dependencies, and historical output
- [x] Recheck upstream API documentation and identify schema and accounting risks
- [ ] Make trade execution explicitly opt-in; default to API-supported simulation
- [ ] Stop estimating portfolio P/L from raw share counts; display API metrics
- [ ] Narrow position/contract deserialization to the fields actually needed
- [ ] Add bounded HTTP timeouts, transient GET retries, and actionable error output
- [ ] Add consistent JSON output and safe watch polling behavior
- [ ] Add regression fixtures, offline unit tests, rustfmt/clippy/test CI
- [ ] Clean accidental generated artifacts from version control
- [ ] Rewrite README and add development/security notes
- [ ] Verify a clean build, unit tests, and representative live read-only API calls
  (Do not mark this complete until actually run and verified.)

## P1 — Expand endpoint coverage

- [ ] Split reusable API client into a library crate; CLI becomes a thin adapter
- [ ] Typed errors and explicit username/ID handling throughout
- [ ] Pagination and robust filtering for markets, bets, positions and history
- [ ] Probability and bulk probability endpoints with structured output
- [ ] Group/topic and user discovery endpoints
- [ ] Order inspection/cancellation and share-selling flows (all safe by default)
- [ ] Live REST compatibility fixtures; capture only non-sensitive public responses
- [ ] Integration tests using a local HTTP mock server (never authenticated live trading)
- [ ] API limits, backoff, observability, and resilience for intermittent networks

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
