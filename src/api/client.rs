use reqwest::{Client, StatusCode, header};
use serde::Serialize;
use std::time::Duration;
use tracing::debug;

use crate::api::models::*;

const BASE_URL: &str = "https://api.manifold.markets/v0";
const MAX_GET_ATTEMPTS: u32 = 3;

#[derive(Debug, thiserror::Error)]
pub enum ApiError {
    #[error("Manifold API returned HTTP {status}: {body}")]
    Http { status: StatusCode, body: String },
    #[error("Invalid Manifold request: {0}")]
    InvalidInput(String),
    #[error("Manifold client configuration error: {0}")]
    Configuration(&'static str),
    #[error("Transport error for {endpoint}: {source}")]
    Transport {
        endpoint: String,
        #[source]
        source: reqwest::Error,
    },
    #[error("Response decoding failed for {endpoint}: {source}")]
    Decode {
        endpoint: String,
        #[source]
        source: reqwest::Error,
    },
}
pub type ApiResult<T> = std::result::Result<T, ApiError>;

impl ApiError {
    pub fn status(&self) -> Option<StatusCode> {
        match self {
            Self::Http { status, .. } => Some(*status),
            _ => None,
        }
    }
}

/// Only a missing username permits falling back to lookup by ID.
pub fn is_not_found(error: &ApiError) -> bool {
    error.status() == Some(StatusCode::NOT_FOUND)
}

fn bounded_limit(n: i32, name: &str) -> ApiResult<()> {
    if !(1..=1000).contains(&n) {
        return Err(ApiError::InvalidInput(format!(
            "{name} must be between 1 and 1000"
        )));
    }
    Ok(())
}

/// Optional filters for the public GET /v0/bets endpoint.
#[derive(Debug, Default, Clone, Copy)]
pub struct BetFilters<'a> {
    pub user_id: Option<&'a str>,
    pub username: Option<&'a str>,
    pub market_id: Option<&'a str>,
    pub contract_slug: Option<&'a str>,
    pub limit: Option<i32>,
    pub before: Option<&'a str>,
    pub after: Option<&'a str>,
    pub before_time: Option<i64>,
    pub after_time: Option<i64>,
    pub kinds: Option<&'a str>,
    pub order: Option<&'a str>,
}

pub struct ManifoldClient {
    client: Client,
    base_url: String,
}

impl ManifoldClient {
    pub fn new(api_key: Option<String>) -> ApiResult<Self> {
        Self::with_base_url(api_key, BASE_URL)
    }

    /// Custom base URLs are primarily intended for testing. Never send real
    /// API credentials to an untrusted URL.
    pub fn with_base_url(api_key: Option<String>, base_url: &str) -> ApiResult<Self> {
        let mut headers = header::HeaderMap::new();
        if let Some(key) = api_key {
            let mut value = header::HeaderValue::from_str(&format!("Key {key}"))
                .map_err(|_| ApiError::Configuration("invalid API key header"))?;
            value.set_sensitive(true);
            headers.insert(header::AUTHORIZATION, value);
        }

        let client = Client::builder()
            .default_headers(headers)
            .connect_timeout(Duration::from_secs(10))
            .timeout(Duration::from_secs(30))
            .build()
            .map_err(|source| ApiError::Transport {
                endpoint: "HTTP client initialization".into(),
                source,
            })?;

        Ok(Self {
            client,
            base_url: base_url.trim_end_matches('/').to_string(),
        })
    }

    /// Retry only idempotent GETs on transient gateway/connectivity errors.
    /// Never retry 429 automatically: callers must reduce request frequency.
    async fn get<T>(&self, endpoint: &str, query: Option<&[(&str, String)]>) -> ApiResult<T>
    where
        T: serde::de::DeserializeOwned,
    {
        let url = format!("{}/{}", self.base_url, endpoint);
        debug!(url, "GET Manifold");
        for attempt in 0..MAX_GET_ATTEMPTS {
            let mut request = self.client.get(&url);
            if let Some(params) = query {
                request = request.query(params);
            }
            match request.send().await {
                Ok(response) => {
                    let status = response.status();
                    if matches!(status.as_u16(), 502..=504) && attempt + 1 < MAX_GET_ATTEMPTS {
                        tokio::time::sleep(Duration::from_millis(400 * 2_u64.pow(attempt))).await;
                        continue;
                    }
                    if !status.is_success() {
                        let body = response.text().await.unwrap_or_default();
                        return Err(ApiError::Http {
                            status,
                            body: body.chars().take(1200).collect(),
                        });
                    }
                    return response
                        .json::<T>()
                        .await
                        .map_err(|source| ApiError::Decode {
                            endpoint: format!("GET /{endpoint}"),
                            source,
                        });
                }
                Err(err)
                    if (err.is_connect() || err.is_timeout()) && attempt + 1 < MAX_GET_ATTEMPTS =>
                {
                    tokio::time::sleep(Duration::from_millis(400 * 2_u64.pow(attempt))).await;
                }
                Err(source) => {
                    return Err(ApiError::Transport {
                        endpoint: format!("GET /{endpoint}"),
                        source,
                    });
                }
            }
        }
        unreachable!("GET retry loop must return or fail")
    }

    /// POSTs are never retried: repeating a trade can create duplicate orders.
    async fn post<T, B>(&self, endpoint: &str, body: B) -> ApiResult<T>
    where
        T: serde::de::DeserializeOwned,
        B: Serialize,
    {
        let url = format!("{}/{}", self.base_url, endpoint);
        debug!(url, "POST Manifold");
        let response = self
            .client
            .post(&url)
            .json(&body)
            .send()
            .await
            .map_err(|source| ApiError::Transport {
                endpoint: format!("POST /{endpoint}"),
                source,
            })?;
        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(ApiError::Http {
                status,
                body: body.chars().take(1200).collect(),
            });
        }
        response
            .json::<T>()
            .await
            .map_err(|source| ApiError::Decode {
                endpoint: format!("POST /{endpoint}"),
                source,
            })
    }

    /// All public users, paged by the final user's ID from the previous page.
    pub async fn list_users(&self, limit: i32, before: Option<&str>) -> ApiResult<Vec<User>> {
        bounded_limit(limit, "User limit")?;
        let mut query = vec![("limit", limit.to_string())];
        if let Some(cursor) = before {
            query.push(("before", cursor.to_owned()));
        }
        self.get("users", Some(&query)).await
    }

    /// Public topics are returned in pages of up to 500, with a time cursor.
    pub async fn list_groups(&self, before_time: Option<i64>) -> ApiResult<Vec<Group>> {
        let query = before_time.map(|n| vec![("beforeTime", n.to_string())]);
        self.get("groups", query.as_deref()).await
    }

    pub async fn get_group_by_slug(&self, slug: &str) -> ApiResult<Group> {
        self.get(&format!("group/{slug}"), None).await
    }

    pub async fn get_me(&self) -> ApiResult<User> {
        self.get("me", None).await
    }

    pub async fn get_user_by_username(&self, username: &str) -> ApiResult<User> {
        self.get(&format!("user/{username}"), None).await
    }

    pub async fn get_user_by_id(&self, id: &str) -> ApiResult<User> {
        self.get(&format!("user/by-id/{id}"), None).await
    }

    pub async fn get_user_portfolio(&self, user_id: &str) -> ApiResult<LivePortfolioMetrics> {
        self.get(
            "get-user-portfolio",
            Some(&[("userId", user_id.to_owned())]),
        )
        .await
    }

    pub async fn get_user_portfolio_history(
        &self,
        user_id: &str,
        period: &str,
    ) -> ApiResult<Vec<PortfolioMetrics>> {
        if !matches!(period, "daily" | "weekly" | "monthly" | "allTime") {
            return Err(ApiError::InvalidInput(
                "period must be daily, weekly, monthly, or allTime".into(),
            ));
        }
        self.get(
            "get-user-portfolio-history",
            Some(&[
                ("userId", user_id.to_owned()),
                ("period", period.to_owned()),
            ]),
        )
        .await
    }

    pub async fn get_user_contract_metrics(
        &self,
        user_id: &str,
        limit: i32,
    ) -> ApiResult<UserContractMetricsResponse> {
        self.get_user_contract_metrics_page(user_id, limit, 0, None, false)
            .await
    }

    /// The response covers only the requested page, not the full account.
    /// Server ordering is "lastBetTime" (default) or "profit".
    pub async fn get_user_contract_metrics_page(
        &self,
        user_id: &str,
        limit: i32,
        offset: u32,
        order: Option<&str>,
        per_answer: bool,
    ) -> ApiResult<UserContractMetricsResponse> {
        bounded_limit(limit, "Position limit")?;
        if order.is_some_and(|o| !matches!(o, "lastBetTime" | "profit")) {
            return Err(ApiError::InvalidInput(
                "order must be lastBetTime or profit".into(),
            ));
        }
        let mut query = vec![
            ("userId", user_id.to_owned()),
            ("limit", limit.to_string()),
            ("offset", offset.to_string()),
            ("perAnswer", per_answer.to_string()),
        ];
        if let Some(o) = order {
            query.push(("order", o.to_owned()));
        }
        self.get("get-user-contract-metrics-with-contracts", Some(&query))
            .await
    }

    pub async fn get_market_positions(
        &self,
        market_id: &str,
        top: Option<i32>,
        bottom: Option<i32>,
    ) -> ApiResult<Vec<ContractMetric>> {
        let mut query = Vec::new();
        if let Some(n) = top {
            query.push(("top", n.to_string()));
        }
        if let Some(n) = bottom {
            query.push(("bottom", n.to_string()));
        }
        self.get(&format!("market/{market_id}/positions"), Some(&query))
            .await
    }

    pub async fn list_markets(
        &self,
        limit: Option<i32>,
        sort: Option<&str>,
        order: Option<&str>,
        before: Option<&str>,
    ) -> ApiResult<Vec<LiteMarket>> {
        self.list_markets_filtered(limit, sort, order, before, None, None)
            .await
    }

    /// Market list filtering supports creator and topic IDs.
    pub async fn list_markets_filtered(
        &self,
        limit: Option<i32>,
        sort: Option<&str>,
        order: Option<&str>,
        before: Option<&str>,
        user_id: Option<&str>,
        group_id: Option<&str>,
    ) -> ApiResult<Vec<LiteMarket>> {
        if let Some(n) = limit {
            bounded_limit(n, "Market limit")?;
        }
        let mut query = Vec::new();
        if let Some(n) = limit {
            query.push(("limit", n.to_string()));
        }
        if let Some(s) = sort {
            query.push(("sort", s.to_owned()));
        }
        if let Some(s) = order {
            query.push(("order", s.to_owned()));
        }
        if let Some(s) = before {
            query.push(("before", s.to_owned()));
        }
        if let Some(id) = user_id {
            query.push(("userId", id.to_owned()));
        }
        if let Some(id) = group_id {
            query.push(("groupId", id.to_owned()));
        }
        self.get("markets", Some(&query)).await
    }

    pub async fn search_markets(
        &self,
        term: &str,
        limit: Option<i32>,
        sort: Option<&str>,
        filter: Option<&str>,
    ) -> ApiResult<Vec<LiteMarket>> {
        self.search_markets_with_cursor(term, limit, sort, filter, None)
            .await
    }

    /// For deep pagination, use sort=newest and beforeTime=last.createdTime.
    pub async fn search_markets_with_cursor(
        &self,
        term: &str,
        limit: Option<i32>,
        sort: Option<&str>,
        filter: Option<&str>,
        before_time: Option<i64>,
    ) -> ApiResult<Vec<LiteMarket>> {
        if before_time.is_some() && sort != Some("newest") {
            return Err(ApiError::InvalidInput(
                "beforeTime is only supported with sort=newest".into(),
            ));
        }
        if let Some(n) = limit {
            bounded_limit(n, "Search limit")?;
        }
        let mut query = vec![("term", term.to_owned())];
        if let Some(n) = limit {
            query.push(("limit", n.to_string()));
        }
        if let Some(s) = sort {
            query.push(("sort", s.to_owned()));
        }
        if let Some(s) = filter {
            query.push(("filter", s.to_owned()));
        }
        if let Some(n) = before_time {
            query.push(("beforeTime", n.to_string()));
        }
        self.get("search-markets", Some(&query)).await
    }

    pub async fn get_market_by_id(&self, id: &str) -> ApiResult<FullMarket> {
        self.get(&format!("market/{id}"), None).await
    }

    pub async fn get_market_by_slug(&self, slug: &str) -> ApiResult<FullMarket> {
        self.get(&format!("slug/{slug}"), None).await
    }

    /// Selling changes a real account only when execute=true. There is no
    /// server-side dryRun documented for /market/{id}/sell.
    pub async fn sell_shares(
        &self,
        market_id: &str,
        outcome: &str,
        shares: Option<f64>,
        answer_id: Option<&str>,
        execute: bool,
    ) -> ApiResult<serde_json::Value> {
        let valid_id = |id: &str| {
            !id.is_empty()
                && id
                    .bytes()
                    .all(|ch| ch.is_ascii_alphanumeric() || ch == b'-' || ch == b'_')
        };
        if !valid_id(market_id) || answer_id.is_some_and(|id| !valid_id(id)) {
            return Err(ApiError::InvalidInput("Invalid market or answer ID".into()));
        }
        if !matches!(outcome, "YES" | "NO") {
            return Err(ApiError::InvalidInput(
                "Sale outcome must be YES or NO".into(),
            ));
        }
        if shares.is_some_and(|n| !n.is_finite() || n <= 0.0) {
            return Err(ApiError::InvalidInput(
                "Share count must be finite and positive".into(),
            ));
        }
        if !execute {
            return Ok(serde_json::json!({
                "marketId": market_id,
                "outcome": outcome,
                "shares": shares,
                "all": shares.is_none(),
                "answerId": answer_id,
                "executed": false,
                "message": "PREVIEW ONLY: nothing sent to Manifold. Add --execute to sell shares."
            }));
        }
        let mut body = serde_json::json!({ "outcome": outcome });
        if let Some(n) = shares {
            body["shares"] = serde_json::json!(n);
        }
        if let Some(id) = answer_id {
            body["answerId"] = serde_json::json!(id);
        }
        self.post(&format!("market/{market_id}/sell"), body).await
    }

    pub async fn market_probability(&self, id: &str) -> ApiResult<serde_json::Value> {
        if id.trim().is_empty() {
            return Err(ApiError::InvalidInput("Market ID cannot be empty".into()));
        }
        self.get(&format!("market/{id}/prob"), None).await
    }

    pub async fn market_probabilities(&self, ids: &[String]) -> ApiResult<serde_json::Value> {
        if ids.is_empty() || ids.len() > 100 || ids.iter().any(|id| id.trim().is_empty()) {
            return Err(ApiError::InvalidInput(
                "Supply 1 to 100 nonempty market IDs".into(),
            ));
        }
        let query: Vec<(&str, String)> = ids.iter().map(|id| ("ids", id.clone())).collect();
        self.get("market-probs", Some(&query)).await
    }

    pub async fn place_bet(
        &self,
        contract_id: &str,
        amount: f64,
        outcome: &str,
        execute: bool,
    ) -> ApiResult<serde_json::Value> {
        validate_bet(amount, outcome)?;
        #[derive(Serialize)]
        #[serde(rename_all = "camelCase")]
        struct BetRequest<'a> {
            amount: f64,
            outcome: &'a str,
            contract_id: &'a str,
            dry_run: bool,
        }
        self.post(
            "bet",
            BetRequest {
                amount,
                outcome,
                contract_id,
                dry_run: !execute,
            },
        )
        .await
    }

    pub async fn list_bets(
        &self,
        user_id: Option<&str>,
        market_id: Option<&str>,
        limit: Option<i32>,
    ) -> ApiResult<Vec<Bet>> {
        self.list_bets_with_cursor(user_id, market_id, limit, None, None)
            .await
    }

    /// Bet ID cursors: before returns older bets, after returns newer bets.
    pub async fn list_bets_with_cursor(
        &self,
        user_id: Option<&str>,
        market_id: Option<&str>,
        limit: Option<i32>,
        before: Option<&str>,
        after: Option<&str>,
    ) -> ApiResult<Vec<Bet>> {
        self.list_bets_filtered(BetFilters {
            user_id,
            market_id,
            limit,
            before,
            after,
            ..BetFilters::default()
        })
        .await
    }

    /// Read-only inspection of bet history and open-limit orders.
    pub async fn list_bets_filtered(&self, filters: BetFilters<'_>) -> ApiResult<Vec<Bet>> {
        if filters.before.is_some() && filters.after.is_some() {
            return Err(ApiError::InvalidInput(
                "Use only one of before or after".into(),
            ));
        }
        if let Some(n) = filters.limit {
            bounded_limit(n, "Bet limit")?;
        }
        if filters.order.is_some_and(|s| !matches!(s, "asc" | "desc")) {
            return Err(ApiError::InvalidInput(
                "Bet order must be asc or desc".into(),
            ));
        }
        if filters.kinds.is_some_and(|s| s != "open-limit") {
            return Err(ApiError::InvalidInput(
                "Only kinds=open-limit is supported".into(),
            ));
        }
        let mut query = Vec::new();
        for (key, value) in [
            ("userId", filters.user_id),
            ("username", filters.username),
            ("contractId", filters.market_id),
            ("contractSlug", filters.contract_slug),
            ("before", filters.before),
            ("after", filters.after),
            ("kinds", filters.kinds),
            ("order", filters.order),
        ] {
            if let Some(value) = value {
                query.push((key, value.to_owned()));
            }
        }
        if let Some(n) = filters.limit {
            query.push(("limit", n.to_string()));
        }
        if let Some(n) = filters.before_time {
            query.push(("beforeTime", n.to_string()));
        }
        if let Some(n) = filters.after_time {
            query.push(("afterTime", n.to_string()));
        }
        self.get("bets", Some(&query)).await
    }

    /// Preview locally by default; the cancellation endpoint has no dry-run support.
    pub async fn cancel_limit_order(
        &self,
        bet_id: &str,
        execute: bool,
    ) -> ApiResult<serde_json::Value> {
        if bet_id.is_empty()
            || !bet_id
                .bytes()
                .all(|ch| ch.is_ascii_alphanumeric() || ch == b'-' || ch == b'_')
        {
            return Err(ApiError::InvalidInput("Invalid bet ID".into()));
        }
        if !execute {
            return Ok(serde_json::json!({
                "betId": bet_id,
                "executed": false,
                "message": "PREVIEW ONLY: nothing sent to Manifold. Add --execute to cancel this order."
            }));
        }
        self.post(&format!("bet/cancel/{bet_id}"), serde_json::json!({}))
            .await
    }
}

pub(crate) fn validate_bet(amount: f64, outcome: &str) -> ApiResult<()> {
    if !amount.is_finite() || amount <= 0.0 {
        return Err(ApiError::InvalidInput(
            "Bet amount must be a finite, positive number".into(),
        ));
    }
    if !matches!(outcome, "YES" | "NO") {
        return Err(ApiError::InvalidInput(
            "Binary bet outcome must be YES or NO".into(),
        ));
    }
    Ok(())
}
