//! 状态自洽性校验

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
    /// TODO(detail-impl):
    ///   - pot ≈ Σ total_bet_this_hand (容差 = max(BB * 0.5, pot * 2%))
    ///   - 任意 seat.stack < 0 → NegativeStack
    ///   - community_cards.len() 与 street 期望值不一致 → CardStreetMismatch
    ///   - 用 (suit, rank) 做唯一性判定（Card 没有 Hash）
    ///   - hero_seat = None → HeroNotConfigured（warning 级）
    ///   - blinds.big_blind <= 0 → BlindsNotConfigured（warning 级）
    pub fn validate(_state: &TableState) -> ValidationResult {
        todo!("StateValidator::validate")
    }
}
