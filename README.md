# Manifoldr

`manifoldr` is a Rust command-line client for the Manifold Markets API. It is intended for terminal-first workflows where you want typed API access, readable tabular output for common lookups, and raw JSON for operations where the full response matters.

The project currently focuses on three areas:

- user lookups and portfolio inspection
- market discovery and market detail retrieval
- bet placement and bet listing

## What The Project Does

The binary talks directly to `https://api.manifold.markets/v0` using `reqwest`, parses responses into Rust types with `serde`, and exposes those operations as a `clap`-based CLI.

Today the CLI supports:

- authenticated user inspection with `user me`
- public user lookup by username or id
- portfolio and portfolio history retrieval
- user and market position inspection
- market listing and search
- full market retrieval by id or slug
- bet placement
- bet listing filtered by user and/or market

Output is intentionally mixed by use case:

- table output for browsing lists and common summaries
- pretty JSON for responses where the full payload is useful

## Features

- async CLI built on `tokio`
- typed API client in `src/api/client.rs`
- response models in `src/api/models.rs`
- environment-variable and flag-based API key support
- `.env` loading via `dotenvy`
- console tracing plus structured JSON log files
- small deserialization tests to catch model drift early

## Requirements

- Rust toolchain
- network access to the Manifold API
- an API key for authenticated endpoints such as `user me` and `bet place`

## Installation

Clone the repository and build it with Cargo:

```bash
cargo build
```

For a release build:

```bash
cargo build --release
```

Run the binary directly from Cargo during development:

```bash
cargo run -- --help
```

## Authentication And Environment

The CLI accepts an API key in two ways:

- `--api-key <KEY>`
- environment variables loaded from the shell or `.env`

Environment handling currently works with:

- `MANIFOLD_API_KEY`
- `API_KEY`

`MANIFOLD_API_KEY` is the documented CLI environment variable because it is wired directly into `clap`. `API_KEY` is also accepted by the runtime fallback in `main.rs`.

Example `.env`:

```dotenv
MANIFOLD_API_KEY=your_manifold_api_key_here
```

Or:

```dotenv
API_KEY=your_manifold_api_key_here
```

## Quick Start

Show top-level help:

```bash
cargo run -- --help
```

Fetch the authenticated user:

```bash
cargo run -- user me
```

Look up a public user:

```bash
cargo run -- user get alice
```

List recent markets:

```bash
cargo run -- market list --limit 20
```

Search markets:

```bash
cargo run -- market search "AI safety" --limit 10
```

Get a market by slug:

```bash
cargo run -- market get some-market-slug --slug
```

Place a bet:

```bash
cargo run -- bet place <MARKET_ID> 50 YES
```

## Command Reference

Top-level command groups:

- `user`
- `market`
- `bet`

### `user`

Operations for user data, portfolio state, and positions.

Commands:

- `manifoldr user me`
  Returns information about the authenticated user and prints it as a table.
- `manifoldr user get <USERNAME_OR_ID>`
  Looks up a user by username or by id. The current implementation guesses id-vs-username based on string length.
- `manifoldr user portfolio <USER_ID>`
  Fetches live portfolio metrics and prints pretty JSON.
- `manifoldr user history <USER_ID> [--period <PERIOD>]`
  Fetches portfolio history and prints a table. Default period is `allTime`.
- `manifoldr user positions <USER_ID> [--limit <N>]`
  Fetches the user's contract metrics and prints positions in a table. Default limit is `10`.

### `market`

Operations for discovering markets and inspecting specific contracts.

Commands:

- `manifoldr market list [--limit <N>] [--sort <SORT>] [--order <ORDER>] [--before <CURSOR>]`
  Lists markets and prints a compact table.
- `manifoldr market search <TERM> [--limit <N>] [--sort <SORT>] [--filter <FILTER>]`
  Searches markets by text query and prints a table.
- `manifoldr market get <ID_OR_SLUG> [--slug]`
  Fetches a full market object and prints pretty JSON. With `--slug`, the argument is treated as a slug instead of a market id.
- `manifoldr market positions <MARKET_ID> [--top <N>] [--bottom <N>]`
  Fetches position data for a market and prints a table.

### `bet`

Operations for placing bets and reviewing existing bets.

Commands:

- `manifoldr bet place <MARKET_ID> <AMOUNT> <OUTCOME>`
  Places a bet and prints the API response as pretty JSON.
- `manifoldr bet list [--user-id <USER_ID>] [--market-id <MARKET_ID>] [--limit <N>]`
  Lists bets and prints them in a table. Default limit is `10`.

## Output Behavior

The CLI uses helper functions in `src/utils.rs` to render common entities as terminal tables:

- users
- markets
- bets
- portfolio history
- positions

Endpoints that return richer objects and where schema fidelity is useful currently print pretty JSON instead:

- `user portfolio`
- `market get`
- `bet place`

This split makes the tool practical both for interactive use and for inspecting raw API responses during development.

## Logging

Logging is initialized at startup in `src/logging.rs`.

Behavior:

- pretty human-readable logs are emitted to the console
- JSON logs are written to `logs/manifoldr.log` with daily rotation
- log filtering is controlled by `RUST_LOG`

Example:

```bash
RUST_LOG=debug cargo run -- market list
```

## Development

Useful commands:

```bash
cargo fmt
cargo check
cargo test
cargo run -- --help
```

Current tests live in `src/tests.rs` and focus on deserializing representative API payloads into the Rust model types.

## Project Layout

This is a single binary crate with a small, direct structure:

```text
.
├── Cargo.toml
├── Cargo.lock
├── README.md
└── src
    ├── main.rs
    ├── cli
    │   └── mod.rs
    ├── api
    │   ├── client.rs
    │   ├── mod.rs
    │   └── models.rs
    ├── logging.rs
    ├── utils.rs
    └── tests.rs
```

### File-By-File

`Cargo.toml`

- crate metadata
- dependency list
- Rust edition declaration

`src/main.rs`

- application entrypoint
- `.env` loading
- logging initialization
- CLI parsing
- API client construction
- command dispatch for `user`, `market`, and `bet`

`src/cli/mod.rs`

- top-level `Cli` struct
- `Commands`, `UserCommands`, `MarketCommands`, and `BetCommands`
- argument and flag definitions via `clap`

`src/api/mod.rs`

- small module barrel that re-exports `ManifoldClient`

`src/api/client.rs`

- HTTP client setup
- authorization header handling
- generic `GET` and `POST` helpers
- concrete API methods for each supported endpoint

`src/api/models.rs`

- Rust data models for API responses
- serde attributes for camelCase payload mapping
- types such as `User`, `LiteMarket`, `FullMarket`, `Bet`, `PortfolioMetrics`, and `ContractMetric`

`src/utils.rs`

- terminal formatting helpers
- `comfy-table` rendering for common result sets

`src/logging.rs`

- tracing subscriber setup
- console and file log layers
- environment-driven filtering with `RUST_LOG`

`src/tests.rs`

- lightweight unit tests covering model deserialization

## Internal Architecture

The code is organized in a straightforward flow:

1. `clap` parses the command line into typed enums in `src/cli/mod.rs`.
2. `main.rs` loads configuration and constructs `ManifoldClient`.
3. Command handlers call the appropriate API method.
4. Responses are either formatted into tables through `src/utils.rs` or emitted as pretty JSON.

This keeps the API layer separate from presentation logic and makes it relatively easy to add new Manifold endpoints without touching unrelated parts of the codebase.

## Current Limitations

- command coverage is intentionally partial and does not expose the full Manifold API
- some output is table-oriented rather than machine-stable JSON
- `user get` uses a simple string-length heuristic to distinguish ids from usernames
- test coverage is currently limited to model deserialization
- API behavior depends on the current Manifold API surface and may require updates if the upstream schema changes

## Extension Points

If you want to grow the project, the natural places are:

- add new typed models in `src/api/models.rs`
- add endpoint methods in `src/api/client.rs`
- add a new `clap` subcommand in `src/cli/mod.rs`
- wire the command into the dispatcher in `src/main.rs`
- add formatter helpers in `src/utils.rs` if the new command should render tables

## Status

The project is already usable as a focused Manifold Markets CLI, especially for interactive inspection and simple trading workflows. The main gaps are breadth of endpoint coverage, more consistent machine-readable output modes, and broader tests around API error handling and response-shape drift.
