//! 状态自洽性校验

use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use tf_core::{SeatId, Street};

use crate::state::TableState;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ValidationIssue {
    PotBetMismatch { pot: f64, total_bets: f64 },
    NegativeStack { seat_id: SeatId, stack: f64 },
    CardStreetMismatch { cards: usize, street: Street },
    DuplicateCards,
    HeroNotConfigured,
    BlindsNotConfigured,
}

#[derive(Debug, Clone)]
pub enum ValidationResult {
    Valid,
    Issues(Vec<ValidationIssue>),
}

pub struct StateValidator;

impl StateValidator {
    pub fn validate(state: &TableState) -> ValidationResult {
        let mut issues = Vec::new();

        let total_bets: f64 = state.seats.iter().map(|s| s.total_bet_this_hand).sum();
        let bb = state.blinds.big_blind.max(0.01);
        let tolerance = (bb * 0.5).max(state.pot.total * 0.02);
        if (state.pot.total - total_bets).abs() > tolerance {
            issues.push(ValidationIssue::PotBetMismatch {
                pot: state.pot.total,
                total_bets,
            });
        }

        for seat in &state.seats {
            if seat.stack < 0.0 {
                issues.push(ValidationIssue::NegativeStack {
                    seat_id: seat.seat_id,
                    stack: seat.stack,
                });
            }
        }

        if state.phase == tf_core::TablePhase::Playing {
            let expected = match state.street {
                Street::Preflop => 0,
                Street::Flop => 3,
                Street::Turn => 4,
                Street::River | Street::Showdown => 5,
            };
            if state.community_cards.len() != expected {
                issues.push(ValidationIssue::CardStreetMismatch {
                    cards: state.community_cards.len(),
                    street: state.street,
                });
            }
        }

        let mut all_cards = state.community_cards.clone();
        if let Some(hole) = &state.hole_cards {
            all_cards.extend_from_slice(hole);
        }
        let unique: HashSet<(tf_core::Suit, tf_core::Rank)> =
            all_cards.iter().map(|c| (c.suit, c.rank)).collect();
        if all_cards.len() != unique.len() {
            issues.push(ValidationIssue::DuplicateCards);
        }

        if state.hero_seat.is_none() {
            issues.push(ValidationIssue::HeroNotConfigured);
        }

        if state.blinds.big_blind <= 0.0 {
            issues.push(ValidationIssue::BlindsNotConfigured);
        }

        if issues.is_empty() {
            ValidationResult::Valid
        } else {
            ValidationResult::Issues(issues)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::{SeatState, TableState};
    use tf_core::{BlindsInfo, Card, Rank, SeatId, SeatStatus, Suit, TablePhase};

    fn make_valid_state() -> TableState {
        let mut state = TableState::initial("test".to_string());
        state.phase = TablePhase::Playing;
        state.street = Street::Preflop;
        state.hero_seat = Some(SeatId::new(0));
        state.blinds = BlindsInfo {
            small_blind: 1.0,
            big_blind: 2.0,
            current_max_bet: 2.0,
            last_raise_size: 2.0,
            ..Default::default()
        };
        state.seats = vec![SeatState {
            seat_id: SeatId::new(0),
            status: SeatStatus::Active,
            stack: 100.0,
            current_bet: 2.0,
            total_bet_this_hand: 2.0,
            last_action: None,
            is_hero: true,
            has_cards: true,
        }];
        state.pot.total = 2.0;
        state.pot.main_pot = 2.0;
        state
    }

    #[test]
    fn test_valid_state() {
        let state = make_valid_state();
        assert!(matches!(
            StateValidator::validate(&state),
            ValidationResult::Valid
        ));
    }

    #[test]
    fn test_negative_stack() {
        let mut state = make_valid_state();
        state.seats[0].stack = -5.0;
        match StateValidator::validate(&state) {
            ValidationResult::Issues(issues) => {
                assert!(issues
                    .iter()
                    .any(|i| matches!(i, ValidationIssue::NegativeStack { .. })));
            }
            _ => panic!("Expected Issues"),
        }
    }

    #[test]
    fn test_card_street_mismatch() {
        let mut state = make_valid_state();
        state.street = Street::Flop;
        state.community_cards = vec![Card {
            suit: Suit::Spades,
            rank: Rank::Ace,
            confidence: 1.0,
        }];
        match StateValidator::validate(&state) {
            ValidationResult::Issues(issues) => {
                assert!(issues
                    .iter()
                    .any(|i| matches!(i, ValidationIssue::CardStreetMismatch { .. })));
            }
            _ => panic!("Expected Issues"),
        }
    }

    #[test]
    fn test_duplicate_cards() {
        let mut state = make_valid_state();
        let card = Card {
            suit: Suit::Spades,
            rank: Rank::Ace,
            confidence: 1.0,
        };
        state.community_cards = vec![card, card];
        state.street = Street::Flop;
        state.community_cards.push(Card {
            suit: Suit::Hearts,
            rank: Rank::King,
            confidence: 1.0,
        });
        match StateValidator::validate(&state) {
            ValidationResult::Issues(issues) => {
                assert!(issues
                    .iter()
                    .any(|i| matches!(i, ValidationIssue::DuplicateCards)));
            }
            _ => panic!("Expected Issues"),
        }
    }

    #[test]
    fn test_hero_not_configured() {
        let mut state = make_valid_state();
        state.hero_seat = None;
        match StateValidator::validate(&state) {
            ValidationResult::Issues(issues) => {
                assert!(issues
                    .iter()
                    .any(|i| matches!(i, ValidationIssue::HeroNotConfigured)));
            }
            _ => panic!("Expected Issues"),
        }
    }

    #[test]
    fn test_blinds_not_configured() {
        let mut state = make_valid_state();
        state.blinds.big_blind = 0.0;
        match StateValidator::validate(&state) {
            ValidationResult::Issues(issues) => {
                assert!(issues
                    .iter()
                    .any(|i| matches!(i, ValidationIssue::BlindsNotConfigured)));
            }
            _ => panic!("Expected Issues"),
        }
    }

    #[test]
    fn test_pot_bet_mismatch() {
        let mut state = make_valid_state();
        state.pot.total = 100.0;
        state.seats[0].total_bet_this_hand = 2.0;
        match StateValidator::validate(&state) {
            ValidationResult::Issues(issues) => {
                assert!(issues
                    .iter()
                    .any(|i| matches!(i, ValidationIssue::PotBetMismatch { .. })));
            }
            _ => panic!("Expected Issues"),
        }
    }
}
