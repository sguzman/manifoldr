# Manifoldr 1.0.0 — MVP acceptance

**Decision:** the established Manifoldr v0.4.0 functionality qualifies for the
first stable MVP (v1.0.0). The ceremony is a semantic-version and documentation
promotion, not a claim of complete Manifold feature coverage or financial accuracy.

## What MVP means here

- **Rust/Linux:** reusable typed Manifold REST client and Linux x86-64 CLI.
- **Market discovery:** browse/search markets, topics/categories, market
  details/probabilities, user and public betting data.
- **Agent access:** one persistent shared Streamable HTTP MCP endpoint with
  nine discoverable read-only tools for market popularity, activity, volume,
  search, category browsing, market details, probabilities and transparent
  non-personalized suggestions. Multiple local/LAN agents consume the same
  endpoint without per-agent servers.
- **Safe writes:** trading commands are CLI-only, explicitly gated by
  `--execute`, with no automatic retry for writes. MCP has no trading or
  authenticated account tools.
- **Intermittent networks:** bounded public GET retries, conservative global
  MCP request pacing, public allowlist-only persistent cache and a strict
  opt-in offline mode for cached CLI reads.
- **Operational:** systemd --user service template in this repository;
  previously installed/active on the user's EndeavourOS host and copied to
  the separate private config-store repository.
- **Quality:** strict rustfmt, Clippy and offline unit/mock HTTP/WebSocket/MCP
  tests, plus GitHub Actions Linux optimized release artifact and checksums.

## Explicit non-goals for v1.0.0

- Complete historical market synchronization or offline private account data.
- Financial statement-grade positions, loan/cash/multi-outcome reconciliation.
- Personalized market suggestions or reproduction of Manifold's private feed.
- Authenticated MCP operations or any trade execution from agents.
- Public-internet hosting, reverse proxy, TLS, user authentication, or remote
  access outside a trusted private LAN.
- Perfect compatibility with future upstream API schema changes.
- More platform-specific packages beyond the GitHub Linux x86-64 release.

These are potential future features, not conditions for MVP graduation. Existing
post-MVP roadmap checkboxes remain open intentionally.

## Final release gate

Release v1.0.0 only after the Rust CI and Linux Binary GitHub workflows pass
for the version-bump commit and the GitHub Release actually contains the
Linux x86-64 tar.gz and SHA256SUMS. Do not claim this gate passed merely
because v0.4.0 previously passed.

## Upgrade an existing service

A machine already running v0.4.0 must install the new binary and **restart**
`manifoldr-mcp.service`. No change to the service unit or the separate
config-store repository is needed. Verify the version and service locally:

    manifoldr --version
    systemctl --user restart manifoldr-mcp.service
    systemctl --user is-active manifoldr-mcp.service

Use `http://127.0.0.1:8877/mcp` locally or
`http://HOST_LAN_IP:8877/mcp` from your trusted LAN.
