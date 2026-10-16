# Manifoldr release policy

Manifoldr uses semantic versioning; v1.0.0 is the **MVP graduation**.
The project began with v0.1.0 and published v0.2.0, v0.3.0 and v0.4.0
during revival on October 15, 2026.

## Stable MVP contract (v1.0.0)

The supported release scope is the Linux x86-64 CLI, Rust library client,
public-only cache and strict offline mode, safe opt-in trading through the CLI,
and a shared read-only Streamable HTTP MCP server with documented discovery
and category tools. Upstream Manifold API and protocol changes may still
require compatibility fixes. See [MVP.md](MVP.md) and [MCP.md](MCP.md).

The MCP server remains unauthenticated and **must be confined to a trusted LAN**
when bound to non-loopback interfaces. No authenticated/financial/trading tools
are exposed over MCP in the MVP. No automatic retries for POST writes.

## Release quality gate

- `cargo fmt --all -- --check`
- `cargo clippy --locked --all-targets -- -D warnings`
- `cargo test --locked --all-targets` (mock HTTP, MCP, WebSocket and unit tests)
- CLI version/help, no-network write previews
- `cargo build --release --locked`
- GitHub Release with Linux x86-64 tarball and SHA256SUMS

The separate public-API smoke probes in Rust CI are explicitly best-effort.
CI never uses real credentials or submits authenticated trades.

The [Linux Binary workflow](.github/workflows/release.yml) automatically
publishes a new `vX.Y.Z` release *once*, from a successful main-branch
build, using the version declared in Cargo.toml. It does not overwrite
previous releases. Never call a release published until its GitHub Release
and checksums are verified.

## Prior verified releases

- [v0.2.0](https://github.com/sguzman/manifoldr/releases/tag/v0.2.0) — revived API client, CLI, WebSocket and safe trading commands
- [v0.3.0](https://github.com/sguzman/manifoldr/releases/tag/v0.3.0) — persistent public cache and offline mode
- [v0.4.0](https://github.com/sguzman/manifoldr/releases/tag/v0.4.0) — official SDK read-only MCP server, nine agent tools, systemd user service template

## v1.0.0 MVP ceremony

Version manifests and release notes prepared October 15, 2026.
Publication is performed by the release workflow only after build/tests pass.
The installed v0.4.0 binary and enabled service continue operating until
the binary is upgraded and `systemctl --user restart manifoldr-mcp.service`
is executed locally. No further `config-store` edits are part of graduation.
