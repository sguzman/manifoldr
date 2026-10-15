# Manifoldr

Rust terminal client for the [Manifold Markets API](https://docs.manifold.markets/api), with tables for interactive inspection and JSON for scripts.

**Status (October 15, 2026):** [Manifoldr v0.2.0](https://github.com/sguzman/manifoldr/releases/tag/v0.2.0) is released with a Linux x86-64 binary, reusable Rust library, WebSocket streaming and opt-in account writes. Both [strict CI](https://github.com/sguzman/manifoldr/actions/runs/38807151269) and the [release build](https://github.com/sguzman/manifoldr/actions/runs/38807151171) passed. Optional offline REST caching and full valuation reconciliation remain future enhancements.

## Verified Linux binary

Download [v0.2.0 for GNU/Linux x86-64](https://github.com/sguzman/manifoldr/releases/download/v0.2.0/manifoldr-linux-x86_64.tar.gz), together with its [SHA256SUMS](https://github.com/sguzman/manifoldr/releases/download/v0.2.0/SHA256SUMS). Verify the checksum before extracting and running the binary.

## Build

Rust stable (edition 2024) and network access are required. The binary is CLI-only and runs on Linux.

    git clone https://github.com/sguzman/manifoldr
    cd manifoldr
    cargo build --locked --release
    cargo run --locked -- --help

Install from the source directory using cargo if preferred:

    cargo install --locked --path .

## Rust library

Manifoldr now exposes a reusable Rust library alongside the CLI binary. Add the repository as a dependency from your own Rust project:

    manifoldr = { git = "https://github.com/sguzman/manifoldr" }

A minimal async example:

    use manifoldr::api::client::ManifoldClient;

    #[tokio::main]
    async fn main() -> anyhow::Result<()> {
        let client = ManifoldClient::new(None)?;
        let markets = client.list_markets(Some(5), None, None, None).await?;
        for market in markets {
            println!("{}", market.question);
        }
        Ok(())
    }

The constructor returns typed `ApiError` failures rather than panicking on invalid HTTP client or API key configuration. Library calls use `ApiResult<T>` and distinguish `Http`, `InvalidInput`, `Configuration`, `Transport` and `Decode` errors; `ApiError::status()` reports HTTP statuses when available.
`ManifoldClient::with_base_url` exists for mock servers and integration testing; **never** pass genuine API credentials to an untrusted host. Public reads do not require credentials.

## Authentication

Public market data requires no API key. To use authenticated commands, create a Manifold API key in your Manifold profile settings. Configure it locally with `MANIFOLD_API_KEY` (preferred) or `API_KEY`.

    export MANIFOLD_API_KEY="YOUR_API_KEY"

The CLI also reads `.env` through `dotenvy`. The file is ignored by Git. Never paste keys into issues, command logs, shell history, or commits. Passing `--api-key` directly is supported for compatibility, but the environment variable is safer because CLI arguments can be visible to other processes.

## Commands

Examples use the installed `manifoldr` binary. During development, prefix commands with `cargo run --locked --`.

| Command | Purpose |
| --- | --- |
| `manifoldr user list --limit 20 [--before USER_ID]` | Browse public users with ID cursor |
| `manifoldr topic list [--before-time UNIX_MILLISECONDS]` | Browse public topics (JSON) |
| `manifoldr topic get TOPIC_SLUG` | Retrieve public topic (JSON) |
| `manifoldr user get USER_ID --id` | Fetch a user directly by immutable ID |
| `manifoldr user portfolio USER_ID --id` | Resolve portfolio directly by immutable ID |
| `manifoldr user history USER_ID --id --period weekly` | Historical portfolio by immutable ID |
| `manifoldr user positions USER_ID --id --limit 25` | Positions by immutable ID |
| `manifoldr user me` | Authenticated user profile |
| `manifoldr user get USERNAME_OR_ID` | Public profile |
| `manifoldr user portfolio [USERNAME_OR_ID]` | Live portfolio JSON |
| `manifoldr user history [USERNAME_OR_ID] --period allTime` | Portfolio time series |
| `manifoldr user positions [USERNAME_OR_ID] --limit 25` | Positions with API-reported P/L |
| `manifoldr user positions USERNAME --offset 25 --order profit --per-answer` | Page positions by server sort and include multiple-choice answers |
| `manifoldr user positions --watch 10` | Refresh positions every 10 seconds |
| `manifoldr market list --limit 20` | Recently created markets |
| `manifoldr market sell MARKET_ID --outcome YES --shares 5` | Preview selling 5 YES shares locally |
| `manifoldr market sell MARKET_ID --outcome YES --shares 5 --execute` | **Sell shares for real** |
| `manifoldr market sell MARKET_ID --outcome NO --all` | Preview selling all NO shares; requires `--execute` to submit |
| `manifoldr market list --user-id USER_ID --group-id TOPIC_ID` | Filter market listing by creator and topic |
| `manifoldr market search "AI safety" --limit 20` | Search markets |
| `manifoldr market search "AI" --topic-slug technology --contract-type BINARY --offset 20` | Filter search by topic, type and page |
| `manifoldr market search "AI" --sort newest --before-time 1706000000000` | Deep search pagination by creation timestamp |
| `manifoldr market probability MARKET_ID` | One market's current probability |
| `manifoldr market get MARKET_ID` | Full market JSON |
| `manifoldr market get MARKET_SLUG --slug` | Get market by URL slug |
| `manifoldr market probabilities MARKET_ID [MARKET_ID ...]` | Bulk probability JSON (1–100 IDs) |
| `manifoldr market positions MARKET_ID --top 20` | Market positions |
| `manifoldr bet list --user-id USER_ID` | Recent bets |
| `manifoldr bet list --username USERNAME --open-limit` | Read-only open limit orders |
| `manifoldr bet list --contract-slug SLUG --before-time UNIX_MS` | Filter bet history |
| `manifoldr bet cancel BET_ID` | Preview cancellation locally (no network) |
| `manifoldr bet cancel BET_ID --execute` | **Cancel a real limit order** |
| `manifoldr bet list --user-id USER_ID --before BET_ID` | Fetch older bets; `--after` fetches newer bets |
| `manifoldr bet place MARKET_ID 5 YES` | **Simulate** a binary bet |

### Real trading is opt-in

`bet place` **simulates by default**, sending Manifold's documented `dryRun: true`. No order is placed unless you explicitly append `--execute`:

    manifoldr bet place MARKET_ID 5 YES --execute

That command **places a real bet** using your authenticated account. This interface supports `YES` and `NO` outcomes for binary markets; passing another outcome or an invalid amount is rejected before contacting Manifold. The client does not automatically retry POST requests, because repeating a request can create a duplicate order. Always confirm account, market ID, and amount before using `--execute`.

### Selling shares (explicit opt-in)

`market sell MARKET_ID --outcome YES --shares 5` previews a sale locally without making any HTTP request. To sell for real, add `--execute`. To request sale of the entire YES or NO position, use `--all` instead of `--shares`; the API receives no `shares` property in that case. Use `--answer-id ANSWER_ID` for multiple-choice market outcomes, as required by Manifold. `--shares` must be finite and greater than zero, and the CLI requires `--outcome YES|NO`. The API has **no documented server-side dry-run** for sales, so the preview is local. The CLI does not verify position ownership or price immediately before execution. An executed sale changes your real account; check the market and amount before using it. Sale POSTs are never retried automatically.

### Order inspection and cancellation

`bet list --open-limit` is read-only. Manifold can include open limit orders from closed or resolved markets. Use `--user-id` or `--username`, `--market-id` or `--contract-slug`, `--before-time`, `--after-time`, and `--order asc|desc` to narrow requests. JSON output exposes optional `orderAmount` and `limitProb` fields.

`bet cancel BET_ID` is a **local preview only**. It sends no request. Manifold does not document a cancellation `dryRun` mode. To actually cancel a limit order, use `bet cancel BET_ID --execute` with authentication. The API is not queried to verify ownership or order status before execution. **Cancellation POSTs are not retried.** If an executed cancellation times out, inspect current order state before attempting it again. Neither real cancellations nor real trades are executed by automated tests.

### JSON output and monitoring

Use `--json` for machine-readable output on commands otherwise displayed as tables:

    manifoldr --json market search "AI" --limit 5
    manifoldr --json user positions USERNAME
    manifoldr --json bet list --limit 10

Commands that already return rich JSON (such as `market get` and `user portfolio`) remain JSON regardless of `--json`. The watch command in JSON mode emits a sequence of pretty-printed JSON documents, **not** one valid JSON array or NDJSON stream; avoid using `--watch` when piping into a JSON parser.

`--watch` requires an interval of at least 2 seconds. For low-bandwidth or unstable connections, prefer 10–30 seconds. Manifold publishes a limit of 500 API requests per minute per IP; do not attempt to circumvent it.

### Portfolio accounting caveat

Position **shares are share counts, not a current market payout valuation**. The CLI displays the API's `invested`, `profit`, and `profitPercent` values rather than estimating P/L from counts. Aggregated totals sum reported position metrics; percentages in the footer are computed from those aggregate amounts. The `PAGE TOTAL` row sums positions in the current API response, not necessarily the complete account portfolio. Request subsequent pages with `--offset`. Use `--all` to include closed/zero-share positions; `--display-limit` limits printed groups only. Do not treat these tables as financial statements.

### Cursor and offset rules

For `market list`, pass `--before` with the final market ID of the prior page. `user list` and `bet list` similarly use entity ID cursors. For `market search`, the API instead requires `--sort newest --before-time` with the last returned market's `createdTime` in milliseconds; it cannot be used with other sort modes. For `topic list`, `--before-time` is a Unix millisecond timestamp. Portfolio positions use numeric `--offset`, `--order lastBetTime|profit`, and optional `--per-answer`. Portfolio history uses `--period daily|weekly|monthly|allTime` and does **not** support pagination in the documented API. These commands fetch one page or period, not the entire account history automatically.

## Live event streaming

Manifoldr now provides a read-only WebSocket client using Manifold's documented event protocol:

    manifoldr stream --topic global/new-bet
    manifoldr stream --topic global/updated-contract --count 10
    manifoldr stream --topic contract/MARKET_ID --topic global/new-contract

The output is **NDJSON**: one compact JSON object per broadcast line. This can be redirected to a local file for later parsing. The client sends protocol keepalive pings every 30 seconds, automatically reconnects and resubscribes after drops, and uses a bounded 1–32 second reconnect delay. Ctrl+C exits. The stream does not send account credentials or issue trades; live server compatibility can change independently of the local WebSocket tests.

## Shell completions

    manifoldr completions fish > manifoldr.fish
    manifoldr completions bash > manifoldr.bash
    manifoldr completions zsh > _manifoldr

## Building release artifacts

Version 0.2.0 is the published Rust CLI/library stabilization release. The [Linux Binary workflow](.github/workflows/release.yml) builds a release-mode x86-64 GNU/Linux binary, archives it and publishes SHA256 checksums as a workflow artifact on every main-branch push. The workflow publishes `v0.2.0` once after its full build/test checks pass, and newer `v*` tags trigger subsequent GitHub Releases. The release workflow runs offline automated tests but intentionally never executes real account writes.

This release does **not** yet include a persistent local REST cache or historical synchronization engine; a stream can already be recorded to NDJSON via shell redirection.

## Public API caching and offline mode (development branch)

The current `main` branch now implements a **public-only on-disk GET cache**. The published v0.2.0 binary predates this feature; it will be available in the next release. The CLI caches allowlisted public reads by default under `$XDG_CACHE_HOME/manifoldr` (or `~/.cache/manifoldr`) for 60 seconds. Repeated requests with matching endpoints and query parameters can be answered without redownloading.

    manifoldr market list --limit 20
    manifoldr --offline market list --limit 20
    manifoldr --cache-ttl 300 market search "AI" --limit 10
    manifoldr --cache-dir /home/user/market-cache --offline topic list
    manifoldr --no-cache market list --limit 20

`--offline` never accesses the API, serves snapshots regardless of age, and reports an explicit cache miss if an endpoint or parameter set has never been fetched. It also prohibits live WebSocket streaming and trading HTTP requests. Cached responses can be stale and **must not be interpreted as live prices**. Trading commands remain network-only; normal read-only endpoints use their usual fresh HTTP requests when caches expire.

Only explicitly allowlisted **public** GET responses (markets, market lookup/probabilities, users and topics) are persisted. Balances, private portfolios, positions, betting history, API credentials, and write responses are **never cached**. No snapshot is downloaded in the background; each API request populates at most one entry.

## Development

Run:

    cargo fmt --all -- --check
    cargo clippy --locked --all-targets -- -D warnings
    cargo test --locked --all-targets

GitHub Actions runs the same offline checks on pushes to `main` and pull requests. Three separate non-blocking public read-only smoke queries exercise markets, users and topics to catch API changes. CI never submits real orders and does not require an API key.

Source layout:

- `src/lib.rs`: public Rust library root
- `src/realtime.rs`: public WebSocket subscriptions, keepalive and reconnect logic
- `tests/http_contract.rs`: local mock-server transport tests (never connect to Manifold)
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
- User identifiers are attempted as usernames first, then as IDs only on HTTP 404. `user get --id` bypasses username lookup.
- API schema changes can still break parsing; please retain non-sensitive fixture samples for regressions.
- Only binary YES/NO bets can be placed. Read-only open-limit inspection, opted-in cancellation and opted-in share sales are available; advanced order creation and multi-answer trading remain unimplemented.
- No independent portfolio price oracle or per-asset/accounting reconciliation.

See [ROADMAP.md](ROADMAP.md), [SECURITY.md](SECURITY.md) and [CHANGELOG.md](CHANGELOG.md).
