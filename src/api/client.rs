use anyhow::{Context, Result, bail};
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
}

/// Only a missing username permits falling back to lookup by ID.
pub fn is_not_found(error: &anyhow::Error) -> bool {
    matches!(
        error.downcast_ref::<ApiError>(),
        Some(ApiError::Http {
            status: StatusCode::NOT_FOUND,
            ..
        })
    )
}

pub struct ManifoldClient {
    client: Client,
    base_url: String,
}

impl ManifoldClient {
    pub fn new(api_key: Option<String>) -> Result<Self> {
        Self::with_base_url(api_key, BASE_URL)
    }

    /// Custom base URLs are primarily intended for testing. Never send real
    /// API credentials to an untrusted URL.
    pub fn with_base_url(api_key: Option<String>, base_url: &str) -> Result<Self> {
        let mut headers = header::HeaderMap::new();
        if let Some(key) = api_key {
            let mut value = header::HeaderValue::from_str(&format!("Key {key}"))
                .context("Invalid API key header format")?;
            value.set_sensitive(true);
            headers.insert(header::AUTHORIZATION, value);
        }

        let client = Client::builder()
            .default_headers(headers)
            .connect_timeout(Duration::from_secs(10))
            .timeout(Duration::from_secs(30))
            .build()
            .context("Failed to construct HTTP client")?;

        Ok(Self {
            client,
            base_url: base_url.trim_end_matches('/').to_string(),
        })
    }

    /// Retry only idempotent GETs on transient gateway/connectivity errors.
    /// Never retry 429 automatically: callers must reduce request frequency.
    async fn get<T>(&self, endpoint: &str, query: Option<&[(&str, String)]>) -> Result<T>
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
                        }
                        .into());
                    }
                    return response
                        .json::<T>()
                        .await
                        .with_context(|| format!("Invalid response from GET /{endpoint}"));
                }
                Err(err)
                    if (err.is_connect() || err.is_timeout()) && attempt + 1 < MAX_GET_ATTEMPTS =>
                {
                    tokio::time::sleep(Duration::from_millis(400 * 2_u64.pow(attempt))).await;
                }
                Err(err) => return Err(err).with_context(|| format!("GET /{endpoint} failed")),
            }
        }
        unreachable!("GET retry loop must return or fail")
    }

    /// POSTs are never retried: repeating a trade can create duplicate orders.
    async fn post<T, B>(&self, endpoint: &str, body: B) -> Result<T>
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
            .with_context(|| format!("POST /{endpoint} failed"))?;
        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(ApiError::Http {
                status,
                body: body.chars().take(1200).collect(),
            }
            .into());
        }
        response
            .json::<T>()
            .await
            .with_context(|| format!("Invalid response from POST /{endpoint}"))
    }

    /// All public users, paged by the final user's ID from the previous page.
    pub async fn list_users(&self, limit: i32, before: Option<&str>) -> Result<Vec<User>> {
        if !(1..=1000).contains(&limit) { bail!("User limit must be between 1 and 1000"); }
        let mut query = vec![("limit", limit.to_string())];
        if let Some(cursor) = before { query.push(("before", cursor.to_owned())); }
        self.get("users", Some(&query)).await
    }

    /// Public topics are returned in pages of up to 500, with a time cursor.
    pub async fn list_groups(&self, before_time: Option<i64>) -> Result<Vec<Group>> {
        let query = before_time.map(|n| vec![("beforeTime", n.to_string())]);
        self.get("groups", query.as_deref()).await
    }

    pub async fn get_group_by_slug(&self, slug: &str) -> Result<Group> {
        self.get(&format!("group/{slug}"), None).await
    }

    pub async fn get_me(&self) -> Result<User> {
        self.get("me", None).await
    }

    pub async fn get_user_by_username(&self, username: &str) -> Result<User> {
        self.get(&format!("user/{username}"), None).await
    }

    pub async fn get_user_by_id(&self, id: &str) -> Result<User> {
        self.get(&format!("user/by-id/{id}"), None).await
    }

    pub async fn get_user_portfolio(&self, user_id: &str) -> Result<LivePortfolioMetrics> {
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
    ) -> Result<Vec<PortfolioMetrics>> {
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
    ) -> Result<UserContractMetricsResponse> {
        self.get(
            "get-user-contract-metrics-with-contracts",
            Some(&[("userId", user_id.to_owned()), ("limit", limit.to_string())]),
        )
        .await
    }

    pub async fn get_market_positions(
        &self,
        market_id: &str,
        top: Option<i32>,
        bottom: Option<i32>,
    ) -> Result<Vec<ContractMetric>> {
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
    ) -> Result<Vec<LiteMarket>> {
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
        self.get("markets", Some(&query)).await
    }

    pub async fn search_markets(
        &self,
        term: &str,
        limit: Option<i32>,
        sort: Option<&str>,
        filter: Option<&str>,
    ) -> Result<Vec<LiteMarket>> {
        self.search_markets_with_cursor(term, limit, sort, filter, None).await
    }

    /// For deep pagination, use sort=newest and beforeTime=last.createdTime.
    pub async fn search_markets_with_cursor(
        &self,
        term: &str,
        limit: Option<i32>,
        sort: Option<&str>,
        filter: Option<&str>,
        before_time: Option<i64>,
    ) -> Result<Vec<LiteMarket>> {
        if before_time.is_some() && sort != Some("newest") {
            bail!("beforeTime is only supported with sort=newest");
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
        if let Some(n) = before_time { query.push(("beforeTime", n.to_string())); }
        self.get("search-markets", Some(&query)).await
    }

    pub async fn get_market_by_id(&self, id: &str) -> Result<FullMarket> {
        self.get(&format!("market/{id}"), None).await
    }

    pub async fn get_market_by_slug(&self, slug: &str) -> Result<FullMarket> {
        self.get(&format!("slug/{slug}"), None).await
    }

    pub async fn market_probability(&self, id: &str) -> Result<serde_json::Value> {
        if id.trim().is_empty() { bail!("Market ID cannot be empty"); }
        self.get(&format!("market/{id}/prob"), None).await
    }

    pub async fn market_probabilities(&self, ids: &[String]) -> Result<serde_json::Value> {
        if ids.is_empty() || ids.len() > 100 || ids.iter().any(|id| id.trim().is_empty()) {
            bail!("Supply 1 to 100 nonempty market IDs");
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
    ) -> Result<serde_json::Value> {
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
    ) -> Result<Vec<Bet>> {
        self.list_bets_with_cursor(user_id, market_id, limit, None, None).await
    }

    /// Bet ID cursors: before returns older bets, after returns newer bets.
    pub async fn list_bets_with_cursor(
        &self,
        user_id: Option<&str>,
        market_id: Option<&str>,
        limit: Option<i32>,
        before: Option<&str>,
        after: Option<&str>,
    ) -> Result<Vec<Bet>> {
        if before.is_some() && after.is_some() { bail!("Use only one of before or after"); }
        if let Some(n) = limit { if !(1..=1000).contains(&n) { bail!("Bet limit must be between 1 and 1000"); } }
        let mut query = Vec::new();
        if let Some(id) = user_id {
            query.push(("userId", id.to_owned()));
        }
        if let Some(id) = market_id {
            query.push(("contractId", id.to_owned()));
        }
        if let Some(n) = limit {
            query.push(("limit", n.to_string()));
        }
        if let Some(id) = before { query.push(("before", id.to_owned())); }
        if let Some(id) = after { query.push(("after", id.to_owned())); }
        self.get("bets", Some(&query)).await
    }
}

pub(crate) fn validate_bet(amount: f64, outcome: &str) -> Result<()> {
    if !amount.is_finite() || amount <= 0.0 {
        bail!("Bet amount must be a finite, positive number");
    }
    if !matches!(outcome, "YES" | "NO") {
        bail!("Binary bet outcome must be YES or NO");
    }
    Ok(())
}
