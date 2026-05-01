#[cfg(test)]
mod tests {
    use crate::api::models::*;
    use serde_json::json;

    #[test]
    fn test_user_deserialization() {
        let data = json!({
            "id": "123",
            "createdTime": 1600000000000i64,
            "name": "Test User",
            "username": "testuser",
            "url": "https://manifold.markets/testuser",
            "balance": 1000.0,
            "totalDeposits": 1000.0
        });

        let user: User = serde_json::from_value(data).unwrap();
        assert_eq!(user.id, "123");
        assert_eq!(user.username, "testuser");
    }

    #[test]
    fn test_lite_market_deserialization() {
        let data = json!({
            "id": "m123",
            "creatorId": "c123",
            "creatorUsername": "creator",
            "creatorName": "Creator",
            "createdTime": 1600000000000i64,
            "question": "Will this test pass?",
            "url": "https://manifold.markets/creator/will-this-pass",
            "outcomeType": "BINARY",
            "mechanism": "cpmm-1",
            "probability": 0.5,
            "volume": 100.0,
            "volume24Hours": 10.0,
            "isResolved": false
        });

        let market: LiteMarket = serde_json::from_value(data).unwrap();
        assert_eq!(market.id, "m123");
        assert_eq!(market.probability, Some(0.5));
    }
}
