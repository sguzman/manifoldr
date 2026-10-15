# Security policy

Manifoldr is a personal, open-source CLI, not an exchange or custody service.

- Keep API keys in a local environment variable or ignored `.env` file. Prefer `MANIFOLD_API_KEY`. Never commit a key.
- Avoid `--api-key` on a shared computer: process arguments may be visible via process listings and shell history.
- Consider keys compromised if they appear in logs, screenshots, issues, or git history; rotate them in Manifold account settings.
- Requests use HTTPS to `api.manifold.markets`. Do not copy keys into other hosts.
- `bet place` is simulated by default. Only `--execute` can submit a live bet.
- No POST is retried automatically; a failed request may still have reached the server. Verify the resulting account state before trying again.
- The API imposes rate limits. The CLI does not attempt to bypass them.
- Tests and CI use no credentials and never execute live trades.
- Treat local logs, position exports and historical fixtures as private even if they do not contain your API key.

For vulnerabilities involving keys or live trading, avoid publishing any sensitive reproduction details.

## Shared MCP server (v0.4.0)

MCP exposes public-read operations only and explicitly drops API credentials. It defaults to loopback; LAN access is opt-in through `--bind 0.0.0.0:8877`. The optional systemd example enables LAN access as requested. Any device that can reach that port can query public data without login. Keep it behind a trusted LAN/firewall, do not forward it onto the public Internet. Incoming browser Origin headers are denied unless explicitly allowlisted.
