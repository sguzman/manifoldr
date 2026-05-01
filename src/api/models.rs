use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct User {
    pub id: String,
    pub created_time: i64,
    pub name: String,
    pub username: String,
    pub url: String,
    pub avatar_url: Option<String>,
    pub bio: Option<String>,
    pub banner_url: Option<String>,
    pub website: Option<String>,
    pub twitter_handle: Option<String>,
    pub discord_handle: Option<String>,
    pub is_bot: Option<bool>,
    pub is_admin: Option<bool>,
    pub is_trustworthy: Option<bool>,
    pub balance: f64,
    pub total_deposits: f64,
    pub last_bet_time: Option<i64>,
    pub current_betting_streak: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LiteMarket {
    pub id: String,
    pub creator_id: String,
    pub creator_username: String,
    pub creator_name: String,
    pub creator_avatar_url: Option<String>,
    pub created_time: i64,
    pub close_time: Option<i64>,
    pub question: String,
    pub url: String,
    pub outcome_type: String,
    pub mechanism: String,
    pub probability: Option<f64>,
    pub pool: Option<serde_json::Value>,
    pub volume: f64,
    pub volume_24_hours: f64,
    pub is_resolved: bool,
    pub resolution: Option<String>,
    pub resolution_time: Option<i64>,
    pub last_updated_time: Option<i64>,
    pub last_bet_time: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FullMarket {
    #[serde(flatten)]
    pub lite: LiteMarket,
    pub answers: Option<Vec<Answer>>,
    pub description: Option<serde_json::Value>,
    pub text_description: String,
    pub group_slugs: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Answer {
    pub id: String,
    pub text: String,
    pub user_id: String,
    pub probability: Option<f64>,
    pub created_time: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Bet {
    pub id: String,
    pub user_id: String,
    pub contract_id: String,
    pub created_time: i64,
    pub amount: f64,
    pub outcome: String,
    pub shares: f64,
    pub is_cancelled: Option<bool>,
    pub is_filled: Option<bool>,
    pub is_redemption: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Group {
    pub id: String,
    pub name: String,
    pub slug: String,
    pub creator_id: String,
    pub created_time: i64,
    pub about: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PortfolioMetrics {
    pub investment_value: f64,
    pub balance: f64,
    pub total_deposits: f64,
    pub loan_total: f64,
    pub timestamp: i64,
    pub profit: Option<f64>,
    pub user_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LivePortfolioMetrics {
    #[serde(flatten)]
    pub metrics: PortfolioMetrics,
    pub daily_profit: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContractMetric {
    pub contract_id: String,
    pub invested: f64,
    pub loan: f64,
    pub payout: f64,
    pub profit: f64,
    pub profit_percent: f64,
    pub total_shares: std::collections::HashMap<String, f64>,
    pub user_id: String,
    pub user_username: String,
    pub user_name: String,
    pub last_bet_time: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserContractMetricsResponse {
    pub metrics_by_contract: std::collections::HashMap<String, Vec<ContractMetric>>,
    pub contracts: Vec<LiteMarket>,
}
