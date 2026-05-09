//! 跨手牌的统计追踪（VPIP / PFR / AF / Hands 等）。
//!
//! MVP 阶段这里只放占位类型，detail-impl 在 v1.1 阶段填充。

use serde::{Deserialize, Serialize};

use tf_core::SeatId;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PlayerStats {
    pub seat_id: SeatId,
    pub hands_played: u64,
    pub vpip: f64,
    pub pfr: f64,
    pub af: f64,
    pub three_bet_pct: f64,
}

pub struct StatsTracker;

impl StatsTracker {
    /// TODO(detail-impl, v1.1)
    pub fn record_action(&mut self, _seat: SeatId, _street: tf_core::Street, _action: &tf_core::ActionType) {
        // no-op in MVP
    }
}
