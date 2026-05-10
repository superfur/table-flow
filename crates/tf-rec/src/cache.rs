//! Recommendation 缓存（必须包含 action_history digest）

use std::collections::HashMap;
use std::sync::Arc;

use parking_lot::RwLock;

use crate::input::RecInput;
use crate::output::RecOutput;

#[derive(Default)]
pub struct RecCache {
    inner: Arc<RwLock<HashMap<String, RecOutput>>>,
    capacity: usize,
}

impl RecCache {
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            inner: Arc::new(RwLock::new(HashMap::with_capacity(capacity))),
            capacity,
        }
    }

    pub fn get(&self, key: &str) -> Option<RecOutput> {
        self.inner.read().get(key).cloned()
    }

    pub fn put(&self, key: String, value: RecOutput) {
        let mut w = self.inner.write();
        if self.capacity > 0 && w.len() >= self.capacity {
            // 简单 LRU 替代：随便丢一个。detail-impl 阶段换成正经 LRU。
            if let Some(k) = w.keys().next().cloned() {
                w.remove(&k);
            }
        }
        w.insert(key, value);
    }

    pub fn clear(&self) {
        self.inner.write().clear();
    }
}

/// Cache key 必须包含 action_history digest，否则同样的 pot/stack 但不同 line
/// 会被错误命中（GTO 推荐对 line 极度敏感）。
///
/// TODO(detail-impl): 用 blake3 / xxh3 输出稳定 hex
pub fn compute_cache_key(input: &RecInput) -> String {
    let history_digest: String = input
        .action_history
        .iter()
        .map(|a| {
            format!(
                "{}:{:?}:{:.2}",
                a.seat_id.0,
                std::mem::discriminant(&a.action),
                a.amount,
            )
        })
        .collect::<Vec<_>>()
        .join("|");

    format!(
        "{:?}|{:?}|p{:.2}|c{:.2}|s{:.2}|m{:.2}|{:?}|n{}|h[{}]",
        input.hole_cards,
        input.community_cards,
        input.pot,
        input.to_call,
        input.stack,
        input.min_raise,
        input.street,
        input.num_opponents,
        history_digest,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::output::RecOutput;
    use tf_core::{Card, Rank, Street, Suit};
    use std::collections::HashMap;

    fn make_input(action_count: usize) -> crate::input::RecInput {
        use crate::input::RecActionRecord;
        use tf_core::{ActionType, SeatId};

        let history: Vec<RecActionRecord> = (0..action_count)
            .map(|i| RecActionRecord {
                seat_id: SeatId::new((i % 3) as u8),
                action: ActionType::Call,
                amount: 50.0 + i as f64,
                street: Street::Preflop,
            })
            .collect();

        crate::input::RecInput {
            hole_cards: [
                Card { suit: Suit::Spades, rank: Rank::Ace, confidence: 1.0 },
                Card { suit: Suit::Hearts, rank: Rank::King, confidence: 1.0 },
            ],
            community_cards: vec![],
            pot: 100.0,
            to_call: 50.0,
            min_raise: 100.0,
            stack: 500.0,
            street: Street::Preflop,
            num_opponents: 3,
            action_history: history,
        }
    }

    fn make_output(action: &str) -> RecOutput {
        RecOutput {
            action: action.to_string(),
            amount: 0.0,
            confidence: 0.8,
            distribution: HashMap::new(),
            ev: 1.0,
            processing_time_ms: 10.0,
        }
    }

    #[test]
    fn test_cache_put_get() {
        let cache = RecCache::with_capacity(10);
        let key = "test_key".to_string();
        let output = make_output("fold");
        cache.put(key.clone(), output.clone());
        let result = cache.get(&key);
        assert!(result.is_some());
        assert_eq!(result.unwrap().action, "fold");
    }

    #[test]
    fn test_cache_miss() {
        let cache = RecCache::with_capacity(10);
        assert!(cache.get("nonexistent").is_none());
    }

    #[test]
    fn test_cache_clear() {
        let cache = RecCache::with_capacity(10);
        cache.put("key1".to_string(), make_output("fold"));
        cache.put("key2".to_string(), make_output("call"));
        cache.clear();
        assert!(cache.get("key1").is_none());
        assert!(cache.get("key2").is_none());
    }

    #[test]
    fn test_cache_capacity_eviction() {
        let cache = RecCache::with_capacity(2);
        cache.put("key1".to_string(), make_output("fold"));
        cache.put("key2".to_string(), make_output("call"));
        cache.put("key3".to_string(), make_output("raise"));
        let results = [
            cache.get("key1").is_some(),
            cache.get("key2").is_some(),
            cache.get("key3").is_some(),
        ];
        let some_count = results.iter().filter(|&&r| r).count();
        assert_eq!(some_count, 2);
    }

    #[test]
    fn test_cache_overwrite() {
        let cache = RecCache::with_capacity(10);
        cache.put("key1".to_string(), make_output("fold"));
        cache.put("key1".to_string(), make_output("call"));
        let result = cache.get("key1").unwrap();
        assert_eq!(result.action, "call");
    }

    #[test]
    fn test_compute_cache_key_deterministic() {
        let input = make_input(2);
        let key1 = compute_cache_key(&input);
        let key2 = compute_cache_key(&input);
        assert_eq!(key1, key2);
    }

    #[test]
    fn test_compute_cache_key_differs_with_history() {
        let input1 = make_input(1);
        let input2 = make_input(2);
        let key1 = compute_cache_key(&input1);
        let key2 = compute_cache_key(&input2);
        assert_ne!(key1, key2);
    }

    #[test]
    fn test_compute_cache_key_includes_cards() {
        let mut input = make_input(0);
        let key1 = compute_cache_key(&input);
        input.hole_cards[0] = Card { suit: Suit::Clubs, rank: Rank::Two, confidence: 1.0 };
        let key2 = compute_cache_key(&input);
        assert_ne!(key1, key2);
    }
}
