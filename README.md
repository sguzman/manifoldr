# Manifoldr

Rust terminal client for the [Manifold Markets API](https://docs.manifold.markets/api), with tables for interactive inspection and JSON for scripts.

**Status (October 2026):** Maintenance revival underway. See [ROADMAP.md](ROADMAP.md) for implemented and planned work and [CHANGELOG.md](CHANGELOG.md) for behavioral changes. The public Manifold API is marked alpha; response schemas may change.

## Build

Rust stable (edition 2024) and network access are required. The binary is CLI-only and runs on Linux.

    git clone https://github.com/sguzman/manifoldr
    cd manifoldr
    cargo build --locked --release
    cargo run --locked -- --help

Install from the source directory using cargo if preferred:

    cargo install --locked --path .

## Authentication

Public market data requires no API key. To use authenticated commands, create a Manifold API key in your Manifold profile settings. Configure it locally with `MANIFOLD_API_KEY` (preferred) or `API_KEY`.

    export MANIFOLD_API_KEY="YOUR_API_KEY"

The CLI also reads `.env` through `dotenvy`. The file is ignored by Git. Never paste keys into issues, command logs, shell history, or commits. Passing `--api-key` directly is supported for compatibility, but the environment variable is safer because CLI arguments can be visible to other processes.

## Commands

Examples use the installed `manifoldr` binary. During development, prefix commands with `cargo run --locked --`.

| Command | Purpose |
| --- | --- |
| `manifoldr user me` | Authenticated user profile |
| `manifoldr user get USERNAME_OR_ID` | Public profile |
| `manifoldr user portfolio [USERNAME_OR_ID]` | Live portfolio JSON |
| `manifoldr user history [USERNAME_OR_ID] --period allTime` | Portfolio time series |
| `manifoldr user positions [USERNAME_OR_ID] --limit 25` | Positions with API-reported P/L |
| `manifoldr user positions --watch 10` | Refresh positions every 10 seconds |
| `manifoldr market list --limit 20` | Recently created markets |
| `manifoldr market search "AI safety" --limit 20` | Search markets |
| `manifoldr market get MARKET_ID` | Full market JSON |
| `manifoldr market get MARKET_SLUG --slug` | Get market by URL slug |
| `manifoldr market probabilities MARKET_ID [MARKET_ID ...]` | Bulk probability JSON (1–100 IDs) |
| `manifoldr market positions MARKET_ID --top 20` | Market positions |
| `manifoldr bet list --user-id USER_ID` | Recent bets |
| `manifoldr bet place MARKET_ID 5 YES` | **Simulate** a binary bet |

### Real trading is opt-in

`bet place` **simulates by default**, sending Manifold's documented `dryRun: true`. No order is placed unless you explicitly append `--execute`:

    manifoldr bet place MARKET_ID 5 YES --execute

That command **places a real bet** using your authenticated account. This interface supports `YES` and `NO` outcomes for binary markets; passing another outcome or an invalid amount is rejected before contacting Manifold. The client does not automatically retry POST requests, because repeating a request can create a duplicate order. Always confirm account, market ID, and amount before using `--execute`.

### JSON output and monitoring

Use `--json` for machine-readable output on commands otherwise displayed as tables:

    manifoldr --json market search "AI" --limit 5
    manifoldr --json user positions USERNAME
    manifoldr --json bet list --limit 10

Commands that already return rich JSON (such as `market get` and `user portfolio`) remain JSON regardless of `--json`. The watch command in JSON mode emits a sequence of pretty-printed JSON documents, **not** one valid JSON array or NDJSON stream; avoid using `--watch` when piping into a JSON parser.

`--watch` requires an interval of at least 2 seconds. For low-bandwidth or unstable connections, prefer 10–30 seconds. Manifold publishes a limit of 500 API requests per minute per IP; do not attempt to circumvent it.

### Portfolio accounting caveat

Position **shares are share counts, not a current market payout valuation**. The CLI displays the API's `invested`, `profit`, and `profitPercent` values rather than estimating P/L from counts. Aggregated totals sum reported position metrics; percentages in the footer are computed from those aggregate amounts. Total rows are for positions retrieved in the current API response, not necessarily the complete account portfolio. Use `--all` to include closed/zero-share positions; `--display-limit` limits printed groups only. Do not treat these tables as financial statements.

## Development

Run:

    cargo fmt --all -- --check
    cargo clippy --locked --all-targets -- -D warnings
    cargo test --locked --all-targets

GitHub Actions runs the same offline checks on pushes to `main` and pull requests. It includes a non-blocking, public, read-only smoke query to catch API changes. CI never submits real orders and does not require an API key.

Source layout:

- `src/cli/mod.rs`: clap command and argument definitions
- `src/api/client.rs`: HTTP client, retry/timeout policy, endpoint calls
- `src/api/models.rs`: serde wire models
- `src/main.rs`: command dispatch
- `src/utils.rs`: terminal tables and position summaries
- `src/tests.rs` and `fixtures/`: offline regression tests
- `src/logging.rs`: console tracing and rotated JSON logs

Console logging is controlled by `RUST_LOG`. File logs appear under `logs/` and are Git-ignored.

Manifold documentation: https://docs.manifold.markets/api

## Known limitations

- Incomplete endpoint coverage, no WebSocket client or persistent offline cache yet.
- User identifiers are attempted as usernames first, then as IDs only on HTTP 404.
- API schema changes can still break parsing; please retain non-sensitive fixture samples for regressions.
- Only binary YES/NO bets are exposed in this CLI; advanced orders, cancellation and multi-answer trading are not yet implemented.
- No independent portfolio price oracle or per-asset/accounting reconciliation.

See [ROADMAP.md](ROADMAP.md), [SECURITY.md](SECURITY.md) and [CHANGELOG.md](CHANGELOG.md).
