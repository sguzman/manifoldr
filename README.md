# Manifoldr

Manifoldr is a Rust CLI client for the Manifold Markets API.

## Intent

Provide a typed, scriptable, terminal-friendly interface to user, market, and betting operations without relying on ad hoc HTTP calls.

## Ambition

The split across API, CLI, logging, and utilities suggests a goal of becoming a practical power-user client for interacting with Manifold from the shell.

## Current Status

The code already supports multiple command groups, environment-based credentials, tracing, and structured output. The project looks usable even though it does not yet have a README of its own.

## Core Capabilities Or Focus Areas

- User commands such as profile, portfolio, history, and positions.
- Market listing, search, lookup, and position inspection.
- Bet placement and listing.
- API key handling via CLI or environment.
- Tracing/logging around API workflows.

## Project Layout

- `src/`: Rust source for the main crate or application entrypoint.
- `Cargo.toml`: crate or workspace manifest and the first place to check for package structure.

## Setup And Requirements

- Rust toolchain.
- Network access to the Manifold API.
- An API key for authenticated operations such as placing bets or reading private data.

## Build / Run / Test Commands

```bash
cargo build
cargo test
cargo run -- --help
```

## Notes, Limitations, Or Known Gaps

- The CLI depends on the current Manifold API surface and credentials model.
- Rate limits, auth failures, and API schema changes are normal operational concerns for this type of tool.

## Next Steps Or Roadmap Hints

- Add a dedicated README command reference and example credential flow.
- Expand tests around API error handling and response-shape drift.
