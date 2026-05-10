//! `BettingRoundEngine` —— 行动顺序 / 回合关闭 / to_call / min_raise 计算。
//!
//! 所有方法都是无状态的纯函数（输入 TableState，输出布尔/数值）。

use tf_core::{ActionType, SeatId, SeatStatus};

use crate::state::TableState;

const EPS: f64 = 0.01;

pub struct BettingRoundEngine;

impl BettingRoundEngine {
    pub fn is_round_complete(state: &TableState) -> bool {
        let active: Vec<&crate::state::SeatState> = state
            .seats
            .iter()
            .filter(|s| matches!(s.status, SeatStatus::Active))
            .collect();

        if active.len() < 2 {
            return true;
        }

        let max_bet = state.blinds.current_max_bet;
        let all_matched = active
            .iter()
            .all(|s| (s.current_bet - max_bet).abs() < EPS);

        let all_acted = active.iter().all(|s| {
            state.action_history.iter().any(|a| {
                a.seat_id == s.seat_id
                    && a.street == state.street
                    && !matches!(a.action, ActionType::PostBlind(_))
            })
        });

        all_matched && all_acted
    }

    pub fn to_call_for(state: &TableState, seat: SeatId) -> f64 {
        let seat_state = match state.seats.iter().find(|s| s.seat_id == seat) {
            Some(s) => s,
            None => return 0.0,
        };
        (state.blinds.current_max_bet - seat_state.current_bet).max(0.0)
    }

    pub fn min_raise(state: &TableState) -> f64 {
        state
            .blinds
            .last_raise_size
            .max(state.blinds.big_blind)
    }

    pub fn total_committed(state: &TableState, seat: SeatId) -> f64 {
        state
            .seats
            .iter()
            .find(|s| s.seat_id == seat)
            .map(|s| s.total_bet_this_hand)
            .unwrap_or(0.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::SeatState;
    use tf_core::{BlindsInfo, SeatId, SeatStatus, Street, TablePhase, ActionType};

    fn make_6max_state() -> TableState {
        let mut state = TableState::initial("test".to_string());
        state.phase = TablePhase::Playing;
        state.street = Street::Preflop;
        state.blinds = BlindsInfo {
            small_blind: 1.0,
            big_blind: 2.0,
            current_max_bet: 2.0,
            last_raise_size: 2.0,
            ..Default::default()
        };
        for i in 0..6u8 {
            state.seats.push(SeatState {
                seat_id: SeatId::new(i),
                status: SeatStatus::Active,
                stack: 100.0,
                current_bet: 0.0,
                total_bet_this_hand: 0.0,
                last_action: None,
                is_hero: i == 0,
                has_cards: true,
            });
        }
        state
    }

    #[test]
    fn test_to_call_for_bb() {
        let state = make_6max_state();
        let to_call = BettingRoundEngine::to_call_for(&state, SeatId::new(0));
        assert!((to_call - 2.0).abs() < 0.01);
    }

    #[test]
    fn test_to_call_for_after_bet() {
        let mut state = make_6max_state();
        state.seats[0].current_bet = 2.0;
        let to_call = BettingRoundEngine::to_call_for(&state, SeatId::new(0));
        assert!((to_call - 0.0).abs() < 0.01);
    }

    #[test]
    fn test_min_raise_default() {
        let state = make_6max_state();
        let mr = BettingRoundEngine::min_raise(&state);
        assert!((mr - 2.0).abs() < 0.01);
    }

    #[test]
    fn test_min_raise_after_3bet() {
        let mut state = make_6max_state();
        state.blinds.last_raise_size = 4.0;
        let mr = BettingRoundEngine::min_raise(&state);
        assert!((mr - 4.0).abs() < 0.01);
    }

    #[test]
    fn test_total_committed() {
        let mut state = make_6max_state();
        state.seats[0].total_bet_this_hand = 10.0;
        let committed = BettingRoundEngine::total_committed(&state, SeatId::new(0));
        assert!((committed - 10.0).abs() < 0.01);
    }

    #[test]
    fn test_is_round_complete_no_actions() {
        let state = make_6max_state();
        assert!(!BettingRoundEngine::is_round_complete(&state));
    }

    #[test]
    fn test_is_round_complete_all_acted_and_matched() {
        let mut state = make_6max_state();
        for seat in &mut state.seats {
            seat.current_bet = 2.0;
        }
        for i in 0..6u8 {
            state.action_history.push(crate::state::ActionRecord {
                seat_id: SeatId::new(i),
                action: ActionType::Call,
                amount: 2.0,
                street: Street::Preflop,
                seq: i as u32 + 1,
                confidence: 0.9,
            });
        }
        assert!(BettingRoundEngine::is_round_complete(&state));
    }

    #[test]
    fn test_is_round_complete_one_folded() {
        let mut state = make_6max_state();
        for seat in &mut state.seats {
            seat.current_bet = 2.0;
        }
        state.seats[0].status = SeatStatus::Folded;
        state.seats[0].current_bet = 0.0;
        for i in 1..6u8 {
            state.action_history.push(crate::state::ActionRecord {
                seat_id: SeatId::new(i),
                action: ActionType::Call,
                amount: 2.0,
                street: Street::Preflop,
                seq: i as u32,
                confidence: 0.9,
            });
        }
        assert!(BettingRoundEngine::is_round_complete(&state));
    }
}
