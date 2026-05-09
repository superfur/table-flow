//! 推荐引擎抽象（trait）+ 从 TableState 构建 RecInput 的纯函数。

use async_trait::async_trait;

use tf_core::{SeatStatus, TfError};
use tf_state::{BettingRoundEngine, TableState};

use crate::input::{RecActionRecord, RecInput};
use crate::output::RecOutput;

#[async_trait]
pub trait RecEngine: Send + Sync {
    async fn recommend(&self, input: RecInput) -> Result<RecOutput, TfError>;

    /// 健康检查
    async fn health(&self) -> Result<(), TfError>;
}

/// 从 TableState 构造 RecInput。
///
/// **前提**：state.hero_seat 已被 HeroDetector 填充。
/// 没有 Hero 时返回 None（调用方不应触发推荐）。
///
/// TODO(detail-impl):
///   - 找到 hero_state
///   - to_call = BettingRoundEngine::to_call_for(state, hero_seat)
///   - min_raise = BettingRoundEngine::min_raise(state)
///   - num_opponents = active 玩家数 - 1
///   - action_history 过滤掉 PostBlind
pub fn build_rec_input(state: &TableState) -> Option<RecInput> {
    let hero = state.hero_seat?;
    let hole = state.hole_cards?;
    let hero_state = state.seats.iter().find(|s| s.seat_id == hero)?;

    let to_call = BettingRoundEngine::to_call_for(state, hero);
    let min_raise = BettingRoundEngine::min_raise(state);

    let num_opponents = state
        .seats
        .iter()
        .filter(|s| matches!(s.status, SeatStatus::Active) && s.seat_id != hero)
        .count();

    let action_history: Vec<RecActionRecord> = state
        .action_history
        .iter()
        .filter(|a| !matches!(a.action, tf_core::ActionType::PostBlind(_)))
        .map(|a| RecActionRecord {
            seat_id: a.seat_id,
            action: a.action.clone(),
            amount: a.amount,
            street: a.street,
        })
        .collect();

    Some(RecInput {
        hole_cards: hole,
        community_cards: state.community_cards.clone(),
        pot: state.pot.total,
        to_call,
        min_raise,
        stack: hero_state.stack,
        street: state.street,
        num_opponents,
        action_history,
    })
}

/// 临时占位：把"从 TableState 构造 + 调用 sidecar"拼起来的便利函数。
/// detail-impl 可以加 cache、超时降级等逻辑。
pub async fn recommend_from_state(
    engine: &dyn RecEngine,
    state: &TableState,
) -> Result<Option<RecOutput>, TfError> {
    let Some(input) = build_rec_input(state) else {
        return Ok(None);
    };
    let out = engine.recommend(input).await?;
    Ok(Some(out))
}
