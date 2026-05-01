use anyhow::{Context, Result};
use reqwest::{header, Client};
use tracing::debug;
use crate::api::models::*;
use serde::Serialize;

pub struct ManifoldClient {
    client: Client,
    base_url: String,
}

impl ManifoldClient {
    pub fn new(api_key: Option<String>) -> Self {
        let mut headers = header::HeaderMap::new();
        if let Some(ref key) = api_key {
            let mut auth_value = header::HeaderValue::from_str(&format!("Key {}", key))
                .expect("Invalid API key format");
            auth_value.set_sensitive(true);
            headers.insert(header::AUTHORIZATION, auth_value);
        }

        let client = Client::builder()
            .default_headers(headers)
            .build()
            .expect("Failed to build HTTP client");

        Self {
            client,
            base_url: "https://api.manifold.markets/v0".to_string(),
        }
    }

    async fn get<T>(&self, endpoint: &str, query: Option<&[( &str, String )]>) -> Result<T> 
    where T: for<'de> serde::Deserialize<'de> {
        let url = format!("{}/{}", self.base_url, endpoint);
        debug!("GET request to: {}", url);
        
        let mut request = self.client.get(&url);
        if let Some(q) = query {
            request = request.query(q);
        }

        let response = request.send().await.context("Failed to send GET request")?;
        
        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!("API error: {}", error_text));
        }

        response.json::<T>().await.context("Failed to parse JSON response")
    }

    async fn post<T, B>(&self, endpoint: &str, body: B) -> Result<T>
    where 
        T: for<'de> serde::Deserialize<'de>,
        B: Serialize,
    {
        let url = format!("{}/{}", self.base_url, endpoint);
        debug!("POST request to: {}", url);

        let response = self.client.post(&url)
            .json(&body)
            .send()
            .await
            .context("Failed to send POST request")?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!("API error: {}", error_text));
        }

        response.json::<T>().await.context("Failed to parse JSON response")
    }

    // User Endpoints
    pub async fn get_me(&self) -> Result<User> {
        self.get("me", None).await
    }

    pub async fn get_user_by_username(&self, username: &str) -> Result<User> {
        self.get(&format!("user/{}", username), None).await
    }

    pub async fn get_user_by_id(&self, id: &str) -> Result<User> {
        self.get(&format!("user/by-id/{}", id), None).await
    }

    pub async fn get_user_portfolio(&self, user_id: &str) -> Result<LivePortfolioMetrics> {
        self.get("get-user-portfolio", Some(&[("userId", user_id.to_string())])).await
    }

    // Market Endpoints
    pub async fn list_markets(&self, limit: Option<i32>, sort: Option<&str>, order: Option<&str>, before: Option<&str>) -> Result<Vec<LiteMarket>> {
        let mut query = Vec::new();
        if let Some(l) = limit { query.push(("limit", l.to_string())); }
        if let Some(s) = sort { query.push(("sort", s.to_string())); }
        if let Some(o) = order { query.push(("order", o.to_string())); }
        if let Some(b) = before { query.push(("before", b.to_string())); }
        
        self.get("markets", Some(&query)).await
    }

    pub async fn search_markets(&self, term: &str, limit: Option<i32>, sort: Option<&str>, filter: Option<&str>) -> Result<Vec<LiteMarket>> {
        let mut query = Vec::new();
        query.push(("term", term.to_string()));
        if let Some(l) = limit { query.push(("limit", l.to_string())); }
        if let Some(s) = sort { query.push(("sort", s.to_string())); }
        if let Some(f) = filter { query.push(("filter", f.to_string())); }

        self.get("search-markets", Some(&query)).await
    }

    pub async fn get_market_by_id(&self, id: &str) -> Result<FullMarket> {
        self.get(&format!("market/{}", id), None).await
    }

    pub async fn get_market_by_slug(&self, slug: &str) -> Result<FullMarket> {
        self.get(&format!("slug/{}", slug), None).await
    }

    // Bet Endpoints
    pub async fn place_bet(&self, contract_id: &str, amount: f64, outcome: &str) -> Result<serde_json::Value> {
        #[derive(Serialize)]
        #[serde(rename_all = "camelCase")]
        struct BetRequest<'a> {
            amount: f64,
            outcome: &'a str,
            contract_id: &'a str,
        }
        
        self.post("bet", BetRequest { amount, outcome, contract_id }).await
    }

    pub async fn list_bets(&self, user_id: Option<&str>, market_id: Option<&str>, limit: Option<i32>) -> Result<Vec<Bet>> {
        let mut query = Vec::new();
        if let Some(u) = user_id { query.push(("userId", u.to_string())); }
        if let Some(m) = market_id { query.push(("contractId", m.to_string())); }
        if let Some(l) = limit { query.push(("limit", l.to_string())); }

        self.get("bets", Some(&query)).await
    }
}
