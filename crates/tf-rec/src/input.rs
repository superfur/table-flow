//! 推荐引擎入参 —— 从 hero 视角看牌局

use serde::{Deserialize, Serialize};

use tf_core::{ActionType, BlindKind, Card, SeatId, Street};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecInput {
    pub hole_cards: [Card; 2],
    pub community_cards: Vec<Card>,
    pub pot: f64,
    pub to_call: f64,
    pub min_raise: f64,
    pub stack: f64,
    pub street: Street,
    pub num_opponents: usize,
    pub action_history: Vec<RecActionRecord>,
}

/// 进入推荐引擎的 action history 条目。
/// 注意：**PostBlind 不进入此列表**，由 `build_rec_input` 过滤掉。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecActionRecord {
    pub seat_id: SeatId,
    pub action: ActionType,
    pub amount: f64,
    pub street: Street,
}

impl RecActionRecord {
    /// 是否是强制盲注（不应进入推荐 history）
    pub fn is_blind(&self) -> bool {
        matches!(
            self.action,
            ActionType::PostBlind(BlindKind::SmallBlind)
                | ActionType::PostBlind(BlindKind::BigBlind)
                | ActionType::PostBlind(BlindKind::Straddle)
                | ActionType::PostBlind(BlindKind::Ante)
        )
    }
}
