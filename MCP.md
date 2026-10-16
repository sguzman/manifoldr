# Manifoldr MCP — one server for every agent

Manifoldr offers a **single shared Streamable HTTP MCP server** using the official Rust MCP SDK. A central process serves all local and LAN agents; they do not launch separate processes. This exposes public market discovery only, not trading or authenticated account data.

## Start locally or on LAN

    cargo install --locked --path .
    manifoldr mcp serve

The default endpoint is **http://127.0.0.1:8877/mcp**. To accept LAN agents:

    manifoldr mcp serve --bind 0.0.0.0:8877

Configure all agents with the same remote MCP URL: **http://HOST_LAN_IP:8877/mcp**. HOST_LAN_IP must be replaced with the desktop's real address. The health endpoint is **/healthz**, returning ok.

## systemd user service

Use the included packaging/systemd/manifoldr-mcp.service. It uses the cargo-installed binary at ~/.cargo/bin/manifoldr and listens on all interfaces as requested. Install once:

    mkdir -p ~/.config/systemd/user
    cp packaging/systemd/manifoldr-mcp.service ~/.config/systemd/user/
    systemctl --user daemon-reload
    systemctl --user enable --now manifoldr-mcp.service

Inspect status with systemctl --user status manifoldr-mcp.service; logs with journalctl --user -u manifoldr-mcp.service -n 50 --no-pager. This is a user service, so it runs under the logged-in user manager. Only if you want it to persist after logout, configure linger separately. Installing it is a local step; committing the unit does not change your machine or config store.

## Available agent tools

| Tool | Purpose |
| --- | --- |
| popular_markets | Browse most-popular-ranked markets with optional category |
| trending_markets | Browse markets by daily-score activity |
| high_volume_markets | Browse markets by 24-hour trading volume |
| search_markets | Filter public markets by query, category, status, type, creator and offset |
| suggested_markets | Deterministic non-personalized blend of popularity, activity and freshness |
| list_categories | List public topics/categories |
| get_category | Fetch category information by topic slug |
| get_market | Full market by ID or slug |
| get_probabilities | Up to 100 market probabilities per query |

Every browse tool supports optional topic_slug, term, contract_type, filter, offset and limit. Open markets are the default. Results have structured machine-readable JSON, including tool-level errors.

Suggested markets is our own transparent rank ensemble, not Manifold's private personalized recommendations. The tool ranks candidates using weights 4/3/2 from most-popular, daily-score, freshness-score. It does not inspect account data.

## LAN access and safeguards

The server is **read-only**. No trade/cancel/sell tools are registered. It does not forward MANIFOLD_API_KEY even when supplied in the environment. Authentication is not required because this is intended for a trusted LAN. Anyone who can reach the port can access the public market API, so do not forward the service to the internet. If firewall rules are used, allow the port only on the trusted local network.

Requests carrying an Origin header are denied unless the exact origin is explicitly allowed, preventing browser DNS rebinding by default. To permit a specific browser client, pass repeated --allow-origin http://HOST:PORT options. Native MCP clients normally omit Origin.

One shared public cache and a global request-pacing lock limit concurrent agents' Manifold traffic. The cache can serve public results for 60 seconds; the tool does not claim cached values are instantaneous live prices.

## Quality checks

The GitHub CI tests initialization, tool discovery, invoking a read-only discovery tool and rejecting a hostile Origin using an ephemeral mock HTTP server. It never executes authenticated transactions.

## Versioning

The agent-facing Streamable HTTP server is introduced in Manifoldr v0.4.0. The systemd unit requires this version or newer; the published v0.3.0 binary does not support the MCP subcommand.

## v1.0.0 MVP upgrade

The v1.0.0 release stabilizes the same read-only MCP feature set introduced in v0.4.0. The existing systemd user-service unit and `/mcp` endpoint remain unchanged. Once the new binary is installed, run `systemctl --user restart manifoldr-mcp.service` to replace the still-running earlier process. Do not expose the unauthenticated endpoint outside a trusted LAN.
