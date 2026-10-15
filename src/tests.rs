#[cfg(test)]
mod tests {
    use crate::api::client::validate_bet;
    use crate::api::models::*;
    use serde_json::json;

    #[test]
    fn user_deserializes_without_optional_fields() {
        let value = json!({
            "id": "123", "createdTime": 1600000000000_i64, "name": "Test",
            "username": "test", "url": "https://manifold.markets/test",
            "balance": 100.0, "totalDeposits": 20.0
        });
        let user: User = serde_json::from_value(value).unwrap();
        assert_eq!(user.id, "123");
    }

    #[test]
    fn market_models_accept_optional_and_future_fields() {
        let value = json!({
            "id": "m123", "creatorId": "c123", "creatorUsername": "creator",
            "creatorName": "Creator", "question": "Will it work?",
            "outcomeType": "BINARY", "mechanism": "cpmm-1",
            "isResolved": false, "futureField": 42
        });
        let lite: LiteMarket = serde_json::from_value(value.clone()).unwrap();
        assert_eq!(lite.volume_24_hours, 0.0);
        let full: FullMarket = serde_json::from_value(value).unwrap();
        assert!(full.text_description.is_empty());
    }

    #[test]
    fn saved_portfolio_fixture_deserializes_without_full_market_models() {
        let value = include_str!("../fixtures/positions-2026-05.json");
        let response: UserContractMetricsResponse = serde_json::from_str(value).unwrap();
        assert_eq!(response.contracts.len(), 1);
        let market = &response.contracts[0];
        assert_eq!(market.id, "ghI9lZO0CP");
        assert_eq!(response.metrics_by_contract[&market.id][0].profit, Some(0.0));
    }

    #[test]
    fn minimal_contract_metric_defaults_missing_share_maps() {
        let metric: ContractMetric = serde_json::from_value(json!({
            "contractId":"test", "userId":"user", "invested":5.0,
            "profit":0.2, "profitPercent":4.0
        }))
        .unwrap();
        assert!(metric.total_shares.is_empty());
        assert!(metric.total_spent.is_empty());
    }

    #[test]
    fn validation_rejects_invalid_trade_inputs() {
        for amount in [0.0, -10.0, f64::NAN, f64::INFINITY] {
            assert!(validate_bet(amount, "YES").is_err());
        }
        assert!(validate_bet(5.0, "no").is_err());
        assert!(validate_bet(5.0, "YES").is_ok());
        assert!(validate_bet(5.0, "NO").is_ok());
    }
}
