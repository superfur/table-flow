//! `BettingRoundEngine` —— 行动顺序 / 回合关闭 / to_call / min_raise 计算。
//!
//! 所有方法都是无状态的纯函数（输入 TableState，输出布尔/数值）。

use tf_core::SeatId;

use crate::state::TableState;

pub struct BettingRoundEngine;

impl BettingRoundEngine {
    /// 当前 street 的下注回合是否已经关闭：
    ///   - 所有未弃牌的玩家在本 street 至少 act 一次（PostBlind 不算）
    ///   - 所有 active 玩家的 current_bet 与 current_max_bet 相等
    ///
    /// TODO(detail-impl)
    pub fn is_round_complete(_state: &TableState) -> bool {
        todo!("BettingRoundEngine::is_round_complete")
    }

    /// hero 视角的 to_call：max(current_max_bet - hero.current_bet, 0)
    /// TODO(detail-impl)
    pub fn to_call_for(_state: &TableState, _seat: SeatId) -> f64 {
        todo!("BettingRoundEngine::to_call_for")
    }

    /// min_raise = max(last_raise_size, big_blind)
    /// TODO(detail-impl)
    pub fn min_raise(_state: &TableState) -> f64 {
        todo!("BettingRoundEngine::min_raise")
    }

    /// 计算给定 seat 在本手牌中的"已投入金额"（用于 side pot 分配）
    /// TODO(detail-impl)
    pub fn total_committed(_state: &TableState, _seat: SeatId) -> f64 {
        todo!("BettingRoundEngine::total_committed")
    }
}
