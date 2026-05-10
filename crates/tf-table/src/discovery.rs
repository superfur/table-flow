//! Table discovery —— 自动发现可识别的扑克客户端窗口

use serde::{Deserialize, Serialize};

use tf_core::TfError;
use tf_vision::WindowInfo;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveredTable {
    pub table_id: String,
    pub window: WindowInfo,
    pub matched_profile_id: Option<String>,
}

pub struct TableDiscovery;

impl TableDiscovery {
    pub async fn scan() -> Result<Vec<DiscoveredTable>, TfError> {
        let windows = tf_vision::enumerate_windows(".*")?;

        let tables: Vec<DiscoveredTable> = windows
            .into_iter()
            .map(|w| {
                let table_id = format!("{:x}-{}", w.handle, simple_hash(&w.title));
                DiscoveredTable {
                    table_id,
                    window: w,
                    matched_profile_id: None,
                }
            })
            .collect();

        Ok(tables)
    }

    pub async fn scan_with_profiles(
        profiles: &[tf_core::CalibrationProfile],
    ) -> Result<Vec<DiscoveredTable>, TfError> {
        let windows = tf_vision::enumerate_windows(".*")?;

        let tables: Vec<DiscoveredTable> = windows
            .into_iter()
            .map(|w| {
                let matched = tf_vision::match_profile(profiles, &w.title, None);
                let table_id = format!("{:x}-{}", w.handle, simple_hash(&w.title));

                DiscoveredTable {
                    table_id,
                    window: w,
                    matched_profile_id: matched.map(|p| p.profile_id.clone()),
                }
            })
            .collect();

        Ok(tables)
    }
}

fn simple_hash(s: &str) -> u64 {
    let mut hash: u64 = 5381;
    for b in s.bytes() {
        hash = hash.wrapping_mul(33).wrapping_add(b as u64);
    }
    hash
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_hash_deterministic() {
        let h1 = simple_hash("PokerStars Table #123");
        let h2 = simple_hash("PokerStars Table #123");
        assert_eq!(h1, h2);
    }

    #[test]
    fn test_simple_hash_differs() {
        let h1 = simple_hash("PokerStars Table #123");
        let h2 = simple_hash("GGPoker Table #456");
        assert_ne!(h1, h2);
    }

    #[tokio::test]
    async fn test_scan_returns_ok() {
        let result = TableDiscovery::scan().await;
        assert!(result.is_ok());
    }
}
