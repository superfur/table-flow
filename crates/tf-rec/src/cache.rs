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
