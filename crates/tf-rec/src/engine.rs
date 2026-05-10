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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sidecar::MockRecEngine;
    use std::collections::HashMap;
    use tf_core::{
        ActionType, BlindsInfo, Card, Rank, SeatId, SeatStatus, Street, Suit, TablePhase,
    };
    use tf_state::{ActionRecord, PotInfo, SeatState};

    fn make_state_with_hero() -> TableState {
        TableState {
            table_id: "test".to_string(),
            phase: TablePhase::Playing,
            street: Street::Preflop,
            hand_number: 1,
            dealer_seat: Some(SeatId::new(1)),
            hero_seat: Some(SeatId::new(0)),
            hole_cards: Some([
                Card { suit: Suit::Spades, rank: Rank::Ace, confidence: 1.0 },
                Card { suit: Suit::Hearts, rank: Rank::King, confidence: 1.0 },
            ]),
            community_cards: vec![],
            pot: PotInfo { main_pot: 100.0, side_pots: vec![], total: 100.0 },
            seats: vec![
                SeatState {
                    seat_id: SeatId::new(0),
                    status: SeatStatus::Active,
                    stack: 500.0,
                    current_bet: 0.0,
                    total_bet_this_hand: 50.0,
                    last_action: None,
                    is_hero: true,
                    has_cards: true,
                },
                SeatState {
                    seat_id: SeatId::new(1),
                    status: SeatStatus::Active,
                    stack: 450.0,
                    current_bet: 0.0,
                    total_bet_this_hand: 50.0,
                    last_action: None,
                    is_hero: false,
                    has_cards: true,
                },
                SeatState {
                    seat_id: SeatId::new(2),
                    status: SeatStatus::Folded,
                    stack: 400.0,
                    current_bet: 0.0,
                    total_bet_this_hand: 0.0,
                    last_action: None,
                    is_hero: false,
                    has_cards: false,
                },
            ],
            action_history: vec![
                ActionRecord {
                    seat_id: SeatId::new(1),
                    action: ActionType::PostBlind(tf_core::BlindKind::SmallBlind),
                    amount: 25.0,
                    street: Street::Preflop,
                    seq: 0,
                    confidence: 1.0,
                },
                ActionRecord {
                    seat_id: SeatId::new(0),
                    action: ActionType::PostBlind(tf_core::BlindKind::BigBlind),
                    amount: 50.0,
                    street: Street::Preflop,
                    seq: 1,
                    confidence: 1.0,
                },
                ActionRecord {
                    seat_id: SeatId::new(2),
                    action: ActionType::Fold,
                    amount: 0.0,
                    street: Street::Preflop,
                    seq: 2,
                    confidence: 1.0,
                },
                ActionRecord {
                    seat_id: SeatId::new(1),
                    action: ActionType::Call,
                    amount: 25.0,
                    street: Street::Preflop,
                    seq: 3,
                    confidence: 1.0,
                },
            ],
            current_player_turn: Some(SeatId::new(0)),
            blinds: BlindsInfo {
                small_blind: 25.0,
                big_blind: 50.0,
                ante: 0.0,
                straddle: 0.0,
                current_max_bet: 50.0,
                last_raise_size: 50.0,
            },
            last_update_ms: 0,
            state_confidence: 1.0,
        }
    }

    #[test]
    fn test_build_rec_input_with_hero() {
        let state = make_state_with_hero();
        let input = build_rec_input(&state).unwrap();
        assert_eq!(input.hole_cards[0].rank, Rank::Ace);
        assert_eq!(input.hole_cards[1].rank, Rank::King);
        assert!(input.community_cards.is_empty());
        assert_eq!(input.pot, 100.0);
        assert_eq!(input.stack, 500.0);
        assert_eq!(input.street, Street::Preflop);
        assert_eq!(input.num_opponents, 1);
        assert_eq!(input.action_history.len(), 2);
        assert!(input.action_history.iter().all(|a| !a.is_blind()));
    }

    #[test]
    fn test_build_rec_input_no_hero() {
        let mut state = make_state_with_hero();
        state.hero_seat = None;
        assert!(build_rec_input(&state).is_none());
    }

    #[test]
    fn test_build_rec_input_no_hole_cards() {
        let mut state = make_state_with_hero();
        state.hole_cards = None;
        assert!(build_rec_input(&state).is_none());
    }

    #[test]
    fn test_build_rec_input_filters_post_blind() {
        let state = make_state_with_hero();
        let input = build_rec_input(&state).unwrap();
        for record in &input.action_history {
            assert!(!matches!(record.action, ActionType::PostBlind(_)));
        }
    }

    #[tokio::test]
    async fn test_recommend_from_state_with_hero() {
        use std::collections::HashMap;

        let state = make_state_with_hero();
        let output = RecOutput {
            action: "raise".to_string(),
            amount: 150.0,
            confidence: 0.75,
            distribution: HashMap::new(),
            ev: 2.3,
            processing_time_ms: 15.0,
        };
        let engine = MockRecEngine::new(output);
        let result = recommend_from_state(&engine, &state).await.unwrap();
        assert!(result.is_some());
        assert_eq!(result.unwrap().action, "raise");
    }

    #[tokio::test]
    async fn test_recommend_from_state_no_hero() {
        let mut state = make_state_with_hero();
        state.hero_seat = None;
        let output = RecOutput {
            action: "fold".to_string(),
            amount: 0.0,
            confidence: 0.5,
            distribution: HashMap::new(),
            ev: 0.0,
            processing_time_ms: 0.0,
        };
        let engine = MockRecEngine::new(output);
        let result = recommend_from_state(&engine, &state).await.unwrap();
        assert!(result.is_none());
    }
}
