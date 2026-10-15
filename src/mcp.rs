//! Stateless, read-only Streamable HTTP MCP server for any local/LAN agents.
//! Uses the official Rust SDK for discovery, protocol negotiation and schemas.
//! Absolutely no authenticated tools or write-capable actions are exposed.
use axum::{
    Router,
    body::Body,
    extract::State,
    http::{Request, StatusCode, header},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::get,
};
use rmcp::{
    handler::server::wrapper::Parameters,
    model::CallToolResult,
    schemars::{self, JsonSchema},
    tool, tool_router,
    transport::streamable_http_server::{
        StreamableHttpServerConfig, StreamableHttpService, session::local::LocalSessionManager,
    },
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    collections::HashMap,
    net::SocketAddr,
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::sync::Mutex;

use crate::api::{
    client::{ManifoldClient, SearchFilters},
    models::LiteMarket,
};

#[derive(Clone)]
pub struct MarketMcp {
    client: ManifoldClient,
    /// Pace every tool call across every connected agent. A suggestion can issue
    /// three upstream public requests; 600ms per call caps worst-case < 500/min.
    pace: Arc<Mutex<Instant>>,
}

impl MarketMcp {
    pub fn new(client: ManifoldClient) -> Self {
        Self {
            client,
            pace: Arc::new(Mutex::new(Instant::now() - Duration::from_millis(600))),
        }
    }

    async fn throttle(&self) {
        let mut last = self.pace.lock().await;
        let remaining = Duration::from_millis(600).saturating_sub(last.elapsed());
        if !remaining.is_zero() {
            tokio::time::sleep(remaining).await;
        }
        *last = Instant::now();
    }

    async fn search(&self, args: BrowseArgs, sort: &str) -> Result<Vec<LiteMarket>, String> {
        validate_browse(&args)?;
        self.throttle().await;
        self.client
            .search_markets_advanced(
                args.term.as_deref().unwrap_or(""),
                SearchFilters {
                    limit: Some(args.limit.unwrap_or(20)),
                    sort: Some(sort),
                    filter: Some(args.filter.as_deref().unwrap_or("open")),
                    offset: args.offset.unwrap_or(0),
                    contract_type: args.contract_type.as_deref(),
                    topic_slug: args.topic_slug.as_deref(),
                    creator_id: args.creator_id.as_deref(),
                    ..SearchFilters::default()
                },
            )
            .await
            .map_err(|err| err.to_string())
    }
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct BrowseArgs {
    /// Maximum returned results: 1..=100 (default 20).
    pub limit: Option<i32>,
    /// Text to search for. Omit to browse markets.
    pub term: Option<String>,
    /// Category slug, e.g. politics, technology, economics.
    pub topic_slug: Option<String>,
    /// Manifold market kind: BINARY, MULTIPLE_CHOICE, POLL, etc.
    pub contract_type: Option<String>,
    /// Filter: open (default), all, closed, resolved, news, etc.
    pub filter: Option<String>,
    /// Offset up to 1000; used for search pagination.
    pub offset: Option<u32>,
    /// Filter by a market creator's immutable user ID.
    pub creator_id: Option<String>,
}

fn validate_browse(a: &BrowseArgs) -> Result<(), String> {
    if !(1..=100).contains(&a.limit.unwrap_or(20)) {
        return Err("limit must be between 1 and 100".into());
    }
    if a.offset.unwrap_or(0) > 1000 {
        return Err("offset must be at most 1000".into());
    }
    if a.term.as_deref().is_some_and(|s| s.len() > 500) {
        return Err("Search term is limited to 500 characters".into());
    }
    for s in [&a.topic_slug, &a.contract_type, &a.filter, &a.creator_id]
        .into_iter()
        .flatten()
    {
        if s.len() > 128 {
            return Err("Filter value is limited to 128 characters".into());
        }
    }
    Ok(())
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SearchArgs {
    #[serde(flatten)]
    pub browse: BrowseArgs,
    /// Ranking from Manifold: most-popular, daily-score, 24-hour-vol, score,
    /// freshness-score, newest, liquidity, or random.
    pub sort: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SuggestedArgs {
    /// Number of candidates to return, 1..=30.
    pub limit: Option<i32>,
    /// Restrict suggestions to a category (topic slug).
    pub topic_slug: Option<String>,
    /// Restrict to market type (e.g. BINARY).
    pub contract_type: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct TopicArgs {
    /// Optional Unix millisecond timestamp cursor, for pagination.
    pub before_time: Option<i64>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SlugArgs {
    /// Public Manifold topic slug.
    pub slug: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct MarketIdArgs {
    /// Market immutable ID, or slug when is_slug is true.
    pub id: String,
    pub is_slug: Option<bool>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct MarketIdsArgs {
    /// 1 to 100 immutable market IDs.
    pub ids: Vec<String>,
}

fn json_result<T: Serialize>(result: Result<T, String>) -> CallToolResult {
    match result {
        Ok(value) => match serde_json::to_value(value) {
            Ok(value) => CallToolResult::structured(json!({"data": value})),
            Err(error) => CallToolResult::structured_error(json!({"error": error.to_string()})),
        },
        Err(error) => CallToolResult::structured_error(json!({"error": error})),
    }
}

/// Deterministic discovery heuristic, never represented as personalized
/// recommendations. Results are scored only by observed API list rankings.
async fn suggest(client: &MarketMcp, args: SuggestedArgs) -> Result<Value, String> {
    let limit = args.limit.unwrap_or(10);
    if !(1..=30).contains(&limit) {
        return Err("limit must be between 1 and 30".into());
    }
    let mut scores: HashMap<String, (LiteMarket, i64, Vec<String>)> = HashMap::new();
    for (sort, weight) in [
        ("most-popular", 4_i64),
        ("daily-score", 3_i64),
        ("freshness-score", 2_i64),
    ] {
        let browse = BrowseArgs {
            limit: Some(30),
            term: None,
            topic_slug: args.topic_slug.clone(),
            contract_type: args.contract_type.clone(),
            filter: Some("open".into()),
            offset: None,
            creator_id: None,
        };
        for (index, market) in client.search(browse, sort).await?.into_iter().enumerate() {
            if market.is_resolved {
                continue;
            }
            let entry = scores
                .entry(market.id.clone())
                .or_insert_with(|| (market.clone(), 0, Vec::new()));
            entry.1 += weight * (30_i64 - index as i64).max(1);
            entry.2.push(sort.to_owned());
        }
    }
    let mut ranked: Vec<_> = scores.into_values().collect();
    ranked.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.id.cmp(&b.0.id)));
    let results: Vec<Value> = ranked.into_iter().take(limit as usize).map(|(market, score, sources)| {
        json!({"market": market, "rankingScore": score, "rankedIn": sources})
    }).collect();
    Ok(json!({
        "method": "deterministic-rank-ensemble",
        "personalized": false,
        "explanation": "Blends public most-popular, daily-score and freshness-score ranks; not Manifold's personalized recommendations",
        "markets": results,
    }))
}

#[tool_router(server_handler)]
impl MarketMcp {
    #[tool(
        description = "Browse popular public Manifold markets, optionally filtered by category/topic slug, market type, or text. Read-only."
    )]
    async fn popular_markets(&self, Parameters(args): Parameters<BrowseArgs>) -> CallToolResult {
        json_result(self.search(args, "most-popular").await)
    }

    #[tool(
        description = "Browse trending public markets by Manifold daily activity score, optionally restricted to category, type, or query. Read-only."
    )]
    async fn trending_markets(&self, Parameters(args): Parameters<BrowseArgs>) -> CallToolResult {
        json_result(self.search(args, "daily-score").await)
    }

    #[tool(
        description = "Browse high-volume public markets by 24-hour trading volume. Supports topic/category filters. Read-only."
    )]
    async fn high_volume_markets(
        &self,
        Parameters(args): Parameters<BrowseArgs>,
    ) -> CallToolResult {
        json_result(self.search(args, "24-hour-vol").await)
    }

    #[tool(
        description = "Search public Manifold markets with query, category, type, status, offset and ranking sort. Read-only."
    )]
    async fn search_markets(&self, Parameters(args): Parameters<SearchArgs>) -> CallToolResult {
        let sort = args.sort.as_deref().unwrap_or("most-popular");
        if !matches!(
            sort,
            "most-popular"
                | "daily-score"
                | "24-hour-vol"
                | "score"
                | "freshness-score"
                | "newest"
                | "liquidity"
                | "random"
                | "last-updated"
                | "close-date"
        ) {
            return json_result::<Value>(Err("Unsupported market sort".into()));
        }
        json_result(self.search(args.browse, sort).await)
    }

    #[tool(
        description = "Suggest interesting public markets using a transparent non-personalized popularity/activity/freshness rank ensemble. Optional category. Read-only."
    )]
    async fn suggested_markets(
        &self,
        Parameters(args): Parameters<SuggestedArgs>,
    ) -> CallToolResult {
        json_result(suggest(self, args).await)
    }

    #[tool(
        description = "List public Manifold topics/categories. Optionally page with before_time (Unix milliseconds)."
    )]
    async fn list_categories(&self, Parameters(args): Parameters<TopicArgs>) -> CallToolResult {
        self.throttle().await;
        json_result(
            self.client
                .list_groups(args.before_time)
                .await
                .map_err(|e| e.to_string()),
        )
    }

    #[tool(description = "Fetch the public metadata for a topic/category by slug.")]
    async fn get_category(&self, Parameters(args): Parameters<SlugArgs>) -> CallToolResult {
        if args.slug.is_empty() || args.slug.len() > 128 || args.slug.contains('/') {
            return json_result::<Value>(Err("Invalid topic slug".into()));
        }
        self.throttle().await;
        json_result(
            self.client
                .get_group_by_slug(&args.slug)
                .await
                .map_err(|e| e.to_string()),
        )
    }

    #[tool(
        description = "Get a Manifold market by ID or slug, including description and answer choices."
    )]
    async fn get_market(&self, Parameters(args): Parameters<MarketIdArgs>) -> CallToolResult {
        if args.id.is_empty() || args.id.len() > 256 || args.id.contains('/') {
            return json_result::<Value>(Err("Invalid market ID or slug".into()));
        }
        self.throttle().await;
        let result = if args.is_slug.unwrap_or(false) {
            self.client.get_market_by_slug(&args.id).await
        } else {
            self.client.get_market_by_id(&args.id).await
        };
        json_result(result.map_err(|e| e.to_string()))
    }

    #[tool(description = "Fetch probabilities for up to 100 public market IDs in one API request.")]
    async fn get_probabilities(
        &self,
        Parameters(args): Parameters<MarketIdsArgs>,
    ) -> CallToolResult {
        self.throttle().await;
        json_result(
            self.client
                .market_probabilities(&args.ids)
                .await
                .map_err(|e| e.to_string()),
        )
    }
}

/// Exact Origin allowlist blocks remote websites from doing DNS rebinding to
/// this LAN service. Native agent clients usually omit Origin entirely.
async fn origin_filter(
    State(allowed): State<Arc<Vec<String>>>,
    request: Request<Body>,
    next: Next,
) -> Response {
    if let Some(value) = request.headers().get(header::ORIGIN) {
        let Ok(origin) = value.to_str() else {
            return StatusCode::FORBIDDEN.into_response();
        };
        if !allowed.iter().any(|item| item == origin) {
            return StatusCode::FORBIDDEN.into_response();
        }
    }
    next.run(request).await
}

pub fn router(client: ManifoldClient, allow_origins: Vec<String>) -> Router {
    let app = MarketMcp::new(client);
    let service = StreamableHttpService::new(
        move || Ok(app.clone()),
        LocalSessionManager::default().into(),
        StreamableHttpServerConfig::default()
            .with_legacy_session_mode(false)
            .with_json_response(true),
    );
    Router::new()
        .route("/healthz", get(|| async { "ok" }))
        .nest_service("/mcp", service)
        .layer(middleware::from_fn_with_state(
            Arc::new(allow_origins),
            origin_filter,
        ))
}

pub async fn serve(
    client: ManifoldClient,
    bind: SocketAddr,
    allowed: Vec<String>,
) -> anyhow::Result<()> {
    for value in &allowed {
        let parsed = url::Url::parse(value)?;
        anyhow::ensure!(
            matches!(parsed.scheme(), "http" | "https")
                && parsed.host_str().is_some()
                && parsed.path() == "/"
                && parsed.query().is_none()
                && parsed.fragment().is_none(),
            "Each --allow-origin must be an exact http(s) origin without a path"
        );
    }
    let listener = tokio::net::TcpListener::bind(bind).await?;
    eprintln!(
        "Read-only Manifold MCP listening on http://{}/mcp",
        listener.local_addr()?
    );
    eprintln!("Use one shared server URL for all local/LAN agents. No auth; no trading tools.");
    axum::serve(listener, router(client, allowed))
        .with_graceful_shutdown(async {
            let _ = tokio::signal::ctrl_c().await;
        })
        .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn validation_rejects_unbounded_discovery() {
        let a = BrowseArgs {
            limit: Some(101),
            term: None,
            topic_slug: None,
            contract_type: None,
            filter: None,
            offset: None,
            creator_id: None,
        };
        assert!(validate_browse(&a).is_err());
    }
}
