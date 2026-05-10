//! `ActionReconstructor` —— 从 stack/pot 变化推导玩家动作。
//!
//! 这是"State Derivation"的核心模块：**不通过 OCR 识别按钮文字**，
//! 而是基于 stack diff + pot diff + seat status change 三路信号交叉验证。

use std::collections::HashSet;

use serde::{Deserialize, Serialize};
use tf_core::{
    ActionSource, ActionType, BlindKind, ReconstructedAction, SeatId, Street,
};

use crate::state::TableState;

const EPS: f64 = 0.01;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReconConfig {
    pub min_stack_change: f64,
    pub min_pot_change: f64,
    pub debounce_ms: u64,
    pub confidence_threshold: f32,
}

impl Default for ReconConfig {
    fn default() -> Self {
        Self {
            min_stack_change: 0.5,
            min_pot_change: 0.5,
            debounce_ms: 80,
            confidence_threshold: 0.6,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ReconInput {
    pub stack_changes: Vec<StackDelta>,
    pub pot_change: Option<PotDelta>,
    pub folded_seats: Vec<SeatId>,
    pub allin_seats: Vec<SeatId>,
}

#[derive(Debug, Clone)]
pub struct StackDelta {
    pub seat_id: SeatId,
    pub delta: f64,
    pub confidence: f32,
}

#[derive(Debug, Clone)]
pub struct PotDelta {
    pub delta: f64,
    pub confidence: f32,
}

pub struct ActionReconstructor {
    pub config: ReconConfig,
}

impl ActionReconstructor {
    pub fn new(config: ReconConfig) -> Self {
        Self { config }
    }

    pub fn reconstruct(
        &self,
        prev_state: &TableState,
        input: &ReconInput,
    ) -> Vec<ReconstructedAction> {
        let mut actions = Vec::new();

        for delta in &input.stack_changes {
            if delta.delta >= -self.config.min_stack_change {
                continue;
            }
            if let Some(action) = self.derive_from_stack_change(prev_state, delta) {
                actions.push(action);
            }
        }

        if let Some(pot_change) = &input.pot_change {
            self.cross_validate_with_pot(&mut actions, pot_change);
        }

        for seat_id in &input.folded_seats {
            actions.push(ReconstructedAction {
                seat_id: *seat_id,
                action_type: ActionType::Fold,
                amount: None,
                street: prev_state.street,
                timestamp_ms: 0,
                confidence: 0.95,
                source: ActionSource::SeatStatusChange,
            });
        }

        for seat_id in &input.allin_seats {
            let stack = prev_state
                .seats
                .iter()
                .find(|s| s.seat_id == *seat_id)
                .map(|s| s.stack)
                .unwrap_or(0.0);
            actions.push(ReconstructedAction {
                seat_id: *seat_id,
                action_type: ActionType::AllIn(stack),
                amount: Some(stack),
                street: prev_state.street,
                timestamp_ms: 0,
                confidence: 0.90,
                source: ActionSource::SeatStatusChange,
            });
        }

        self.deduplicate(&mut actions);
        actions
    }

    fn derive_from_stack_change(
        &self,
        prev_state: &TableState,
        change: &StackDelta,
    ) -> Option<ReconstructedAction> {
        let amount = change.delta.abs();
        let seat = prev_state.seats.get(change.seat_id.0 as usize)?;
        let to_call = self.compute_to_call(prev_state);
        let prev_bet = seat.current_bet;
        let new_bet = prev_bet + amount;
        let bb = prev_state.blinds.big_blind;

        if (seat.stack - amount).abs() < EPS || seat.stack - amount <= 0.0 {
            return Some(ReconstructedAction {
                seat_id: change.seat_id,
                action_type: ActionType::AllIn(seat.stack),
                amount: Some(seat.stack),
                street: prev_state.street,
                timestamp_ms: 0,
                confidence: change.confidence * 0.9,
                source: ActionSource::StackDiff,
            });
        }

        if prev_state.street == Street::Preflop
            && prev_state.action_history.is_empty()
            && (amount - prev_state.blinds.small_blind).abs() < EPS
        {
            return Some(ReconstructedAction {
                seat_id: change.seat_id,
                action_type: ActionType::PostBlind(BlindKind::SmallBlind),
                amount: Some(amount),
                street: prev_state.street,
                timestamp_ms: 0,
                confidence: change.confidence * 0.9,
                source: ActionSource::StackDiff,
            });
        }

        if prev_state.street == Street::Preflop
            && prev_state
                .action_history
                .iter()
                .filter(|a| matches!(a.action, ActionType::PostBlind(_)))
                .count()
                < 2
            && (amount - bb).abs() < EPS
            && to_call < EPS
        {
            return Some(ReconstructedAction {
                seat_id: change.seat_id,
                action_type: ActionType::PostBlind(BlindKind::BigBlind),
                amount: Some(amount),
                street: prev_state.street,
                timestamp_ms: 0,
                confidence: change.confidence * 0.9,
                source: ActionSource::StackDiff,
            });
        }

        let action_type = if to_call > EPS && (new_bet - to_call).abs() < EPS {
            ActionType::Call
        } else if to_call < EPS && new_bet > EPS {
            ActionType::Bet(amount)
        } else if to_call > EPS && new_bet > to_call + EPS {
            ActionType::Raise(new_bet)
        } else {
            return None;
        };

        Some(ReconstructedAction {
            seat_id: change.seat_id,
            action_type,
            amount: Some(amount),
            street: prev_state.street,
            timestamp_ms: 0,
            confidence: change.confidence * 0.9,
            source: ActionSource::StackDiff,
        })
    }

    fn cross_validate_with_pot(
        &self,
        actions: &mut Vec<ReconstructedAction>,
        pot_change: &PotDelta,
    ) {
        let total_action_amount: f64 = actions.iter().filter_map(|a| a.amount).sum();

        if (total_action_amount - pot_change.delta).abs() < 2.0 {
            for action in actions.iter_mut() {
                action.confidence = (action.confidence + 0.1).min(1.0);
                action.source = ActionSource::Combined;
            }
        }
    }

    fn compute_to_call(&self, state: &TableState) -> f64 {
        state.blinds.current_max_bet
    }

    fn deduplicate(&self, actions: &mut Vec<ReconstructedAction>) {
        let mut seen: HashSet<(SeatId, Street, std::mem::Discriminant<ActionType>)> =
            HashSet::new();
        actions.retain(|a| {
            seen.insert((a.seat_id, a.street, std::mem::discriminant(&a.action_type)))
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::{SeatState, TableState};
    use tf_core::{BlindsInfo, SeatStatus, TablePhase};

    fn make_state_with_blinds() -> TableState {
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
        state.seats = (0..6u8)
            .map(|i| SeatState {
                seat_id: SeatId::new(i),
                status: if i < 4 {
                    SeatStatus::Active
                } else {
                    SeatStatus::Empty
                },
                stack: 100.0,
                current_bet: 0.0,
                total_bet_this_hand: 0.0,
                last_action: None,
                is_hero: i == 0,
                has_cards: i < 4,
            })
            .collect();
        state
    }

    #[test]
    fn test_derive_call() {
        let mut state = make_state_with_blinds();
        state.seats[0].current_bet = 0.0;
        let recon = ActionReconstructor::new(ReconConfig::default());
        let input = ReconInput {
            stack_changes: vec![StackDelta {
                seat_id: SeatId::new(0),
                delta: -2.0,
                confidence: 0.9,
            }],
            pot_change: Some(PotDelta {
                delta: 2.0,
                confidence: 0.9,
            }),
            folded_seats: vec![],
            allin_seats: vec![],
        };
        let actions = recon.reconstruct(&state, &input);
        assert_eq!(actions.len(), 1);
        assert!(matches!(actions[0].action_type, ActionType::Call));
    }

    #[test]
    fn test_derive_bet() {
        let mut state = make_state_with_blinds();
        state.blinds.current_max_bet = 0.0;
        let recon = ActionReconstructor::new(ReconConfig::default());
        let input = ReconInput {
            stack_changes: vec![StackDelta {
                seat_id: SeatId::new(0),
                delta: -6.0,
                confidence: 0.9,
            }],
            pot_change: None,
            folded_seats: vec![],
            allin_seats: vec![],
        };
        let actions = recon.reconstruct(&state, &input);
        assert_eq!(actions.len(), 1);
        assert!(matches!(actions[0].action_type, ActionType::Bet(v) if (v - 6.0).abs() < 0.01));
    }

    #[test]
    fn test_derive_raise() {
        let mut state = make_state_with_blinds();
        state.seats[0].current_bet = 0.0;
        let recon = ActionReconstructor::new(ReconConfig::default());
        let input = ReconInput {
            stack_changes: vec![StackDelta {
                seat_id: SeatId::new(0),
                delta: -8.0,
                confidence: 0.9,
            }],
            pot_change: None,
            folded_seats: vec![],
            allin_seats: vec![],
        };
        let actions = recon.reconstruct(&state, &input);
        assert_eq!(actions.len(), 1);
        assert!(matches!(actions[0].action_type, ActionType::Raise(v) if (v - 8.0).abs() < 0.01));
    }

    #[test]
    fn test_derive_allin() {
        let mut state = make_state_with_blinds();
        state.seats[0].stack = 10.0;
        let recon = ActionReconstructor::new(ReconConfig::default());
        let input = ReconInput {
            stack_changes: vec![StackDelta {
                seat_id: SeatId::new(0),
                delta: -10.0,
                confidence: 0.9,
            }],
            pot_change: None,
            folded_seats: vec![],
            allin_seats: vec![],
        };
        let actions = recon.reconstruct(&state, &input);
        assert_eq!(actions.len(), 1);
        assert!(matches!(actions[0].action_type, ActionType::AllIn(_)));
    }

    #[test]
    fn test_derive_fold_from_seat_status() {
        let state = make_state_with_blinds();
        let recon = ActionReconstructor::new(ReconConfig::default());
        let input = ReconInput {
            stack_changes: vec![],
            pot_change: None,
            folded_seats: vec![SeatId::new(1)],
            allin_seats: vec![],
        };
        let actions = recon.reconstruct(&state, &input);
        assert_eq!(actions.len(), 1);
        assert!(matches!(actions[0].action_type, ActionType::Fold));
    }

    #[test]
    fn test_cross_validate_bumps_confidence() {
        let state = make_state_with_blinds();
        let recon = ActionReconstructor::new(ReconConfig::default());
        let input = ReconInput {
            stack_changes: vec![StackDelta {
                seat_id: SeatId::new(0),
                delta: -2.0,
                confidence: 0.9,
            }],
            pot_change: Some(PotDelta {
                delta: 2.0,
                confidence: 0.9,
            }),
            folded_seats: vec![],
            allin_seats: vec![],
        };
        let actions = recon.reconstruct(&state, &input);
        assert_eq!(actions.len(), 1);
        assert!(actions[0].confidence > 0.9 * 0.9);
        assert!(matches!(actions[0].source, ActionSource::Combined));
    }

    #[test]
    fn test_deduplicate() {
        let state = make_state_with_blinds();
        let recon = ActionReconstructor::new(ReconConfig::default());
        let input = ReconInput {
            stack_changes: vec![StackDelta {
                seat_id: SeatId::new(0),
                delta: -2.0,
                confidence: 0.9,
            }],
            pot_change: None,
            folded_seats: vec![SeatId::new(1)],
            allin_seats: vec![],
        };
        let actions = recon.reconstruct(&state, &input);
        assert_eq!(actions.len(), 2);
        let has_call = actions.iter().any(|a| matches!(a.action_type, ActionType::Call));
        let has_fold = actions.iter().any(|a| matches!(a.action_type, ActionType::Fold));
        assert!(has_call);
        assert!(has_fold);
    }

    #[test]
    fn test_small_blind_detection() {
        let mut state = make_state_with_blinds();
        state.blinds.current_max_bet = 0.0;
        state.action_history.clear();
        let recon = ActionReconstructor::new(ReconConfig::default());
        let input = ReconInput {
            stack_changes: vec![StackDelta {
                seat_id: SeatId::new(1),
                delta: -1.0,
                confidence: 0.9,
            }],
            pot_change: None,
            folded_seats: vec![],
            allin_seats: vec![],
        };
        let actions = recon.reconstruct(&state, &input);
        assert_eq!(actions.len(), 1);
        assert!(matches!(
            actions[0].action_type,
            ActionType::PostBlind(BlindKind::SmallBlind)
        ));
    }
}
