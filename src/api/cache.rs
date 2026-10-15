//! Public-read cache. The allowlist excludes credentials, balances, portfolios,
//! positions, bets, and every mutation. Offline mode never performs HTTP I/O.
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use std::{path::PathBuf, time::{Duration, SystemTime, UNIX_EPOCH}};

#[derive(Debug, Clone)]
pub struct PublicCache {
    directory: PathBuf,
    ttl: Duration,
    offline: bool,
}

#[derive(Serialize, Deserialize)]
struct Snapshot {
    key: String,
    saved_at: u64,
    value: serde_json::Value,
}

fn seconds_since_epoch() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs()
}

fn stable_hash(key: &str) -> u64 {
    // Fixed-seed FNV-1a. A snapshot also embeds and verifies the full key.
    key.bytes().fold(0xcbf29ce484222325_u64, |hash, byte| {
        (hash ^ u64::from(byte)).wrapping_mul(0x100000001b3)
    })
}

/// Narrow allowlist: even if a user is authenticated, private responses stay
/// out of the cache. Unknown endpoints are not cacheable by default.
pub fn is_public_endpoint(endpoint: &str) -> bool {
    matches!(endpoint, "markets" | "search-markets" | "users" | "groups"
        | "market-probs")
        || endpoint.starts_with("market/")
            && (endpoint.ends_with("/prob")
                || !endpoint["market/".len()..].contains('/'))
        || endpoint.starts_with("slug/")
            && !endpoint["slug/".len()..].contains('/')
        || endpoint.starts_with("group/")
            && !endpoint["group/".len()..].contains('/')
        || endpoint.starts_with("user/")
            && (endpoint.starts_with("user/by-id/") || !endpoint["user/".len()..].contains('/'))
}

impl PublicCache {
    pub fn new(directory: PathBuf, ttl: Duration, offline: bool) -> Self {
        Self { directory, ttl, offline }
    }

    pub fn offline(&self) -> bool {
        self.offline
    }

    fn file(&self, key: &str) -> PathBuf {
        self.directory.join(format!("{:016x}.json", stable_hash(key)))
    }

    pub async fn read<T: DeserializeOwned>(&self, key: &str) -> Option<T> {
        let bytes = tokio::fs::read(self.file(key)).await.ok()?;
        let stored: Snapshot = serde_json::from_slice(&bytes).ok()?;
        if stored.key != key {
            return None;
        }
        if !self.offline && seconds_since_epoch().saturating_sub(stored.saved_at) >= self.ttl.as_secs() {
            return None;
        }
        serde_json::from_value(stored.value).ok()
    }

    pub async fn write<T: Serialize>(&self, key: &str, data: &T) {
        let Ok(value) = serde_json::to_value(data) else { return; };
        let record = Snapshot { key: key.to_owned(), saved_at: seconds_since_epoch(), value };
        let Ok(bytes) = serde_json::to_vec(&record) else { return; };
        if tokio::fs::create_dir_all(&self.directory).await.is_err() {
            return;
        }
        let dest = self.file(key);
        let stamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_nanos();
        let tmp = self.directory.join(format!(".{}-{}-{stamp}.tmp", std::process::id(), stable_hash(key)));
        if tokio::fs::write(&tmp, bytes).await.is_ok() {
            if tokio::fs::rename(&tmp, dest).await.is_err() {
                let _ = tokio::fs::remove_file(&tmp).await;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::is_public_endpoint;
    #[test]
    fn explicit_public_allowlist_never_includes_private_data_or_bets() {
        for name in ["markets", "search-markets", "market/id1", "market/id1/prob",
                     "slug/foo", "users", "user/by-id/abc", "user/name", "groups"] {
            assert!(is_public_endpoint(name), "{name}");
        }
        for name in ["me", "bets", "get-user-portfolio",
                     "get-user-portfolio-history", "get-user-contract-metrics-with-contracts",
                     "market/id1/positions", "bet/cancel/x", "market/id1/sell"] {
            assert!(!is_public_endpoint(name), "{name}");
        }
    }
}
