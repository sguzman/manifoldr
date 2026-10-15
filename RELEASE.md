# Release procedure

Manifoldr follows semantic versions beginning with v0.2.0 for the revived
Rust library and CLI. Minor versions may introduce new endpoints; review
the changelog and the upstream alpha API on each upgrade.

## Quality gate

The strict Rust CI workflow must pass: rustfmt, Clippy with warnings denied,
all unit/HTTP/WebSocket mock tests, command help and local no-network previews.
Live public API smoke probes are best-effort. Authenticated trading and account
operations are **not** executed by automation.

## Packaging

The Linux Binary GitHub Actions workflow runs on main, tags matching v*,
and manual dispatch. It builds a release-mode GNU/Linux x86-64 binary with
Cargo.lock and packages a tar.gz with a SHA256SUMS file. Its workflow artifact
is available directly from the run, even without a GitHub Release.

[v0.2.0](https://github.com/sguzman/manifoldr/releases/tag/v0.2.0) was published on 2026-10-15 with a Linux x86-64 tarball and SHA256SUMS. The workflow runs formatting, Clippy, tests and a release-mode build, publishing the release only if it does not already exist. It never overwrites an existing release. Newer releases
continue to be published only from approved version tags.
Do not cut releases from failing CI or publish API credentials.

## Scope

The 0.2.0 release includes the API library, terminal CLI, safe opt-in account
writes, public streaming and shell completions. Durable offline REST caching,
full financial reconciliation, and platform-specific packaging remain optional
future enhancements. Do not imply those exist until implemented and tested.
