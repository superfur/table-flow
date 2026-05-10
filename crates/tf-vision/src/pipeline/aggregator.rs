//! FeatureAggregator —— 合并各 detector 输出

use tf_core::{Street, TableId};

use crate::features::{ExtractedFeatures, RawFeatures};

pub struct FeatureAggregator {
    pub table_id: TableId,
}

impl FeatureAggregator {
    pub fn new(table_id: TableId) -> Self {
        Self { table_id }
    }

    pub fn merge(&self, raw: RawFeatures) -> ExtractedFeatures {
        let street = match raw.cards.community_cards.len() {
            0 => Street::Preflop,
            3 => Street::Flop,
            4 => Street::Turn,
            _ => Street::River,
        };

        ExtractedFeatures {
            table_id: self.table_id.clone(),
            timestamp_ms: raw.timestamp_ms,
            hole_cards: raw.cards.hole_cards,
            community_cards: raw.cards.community_cards,
            street,
            stack_changes: raw.stacks,
            pot_change: raw.pot,
            seat_changes: raw.seats,
            dealer_seat: raw.dealer,
            hero_seat: raw.hero,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::features::CardDetectionResult;
    use tf_core::{Card, Rank, Suit};

    #[test]
    fn test_merge_preflop() {
        let agg = FeatureAggregator::new("test".into());
        let raw = RawFeatures {
            cards: CardDetectionResult {
                hole_cards: Some([
                    Card { suit: Suit::Spades, rank: Rank::Ace, confidence: 0.99 },
                    Card { suit: Suit::Hearts, rank: Rank::King, confidence: 0.99 },
                ]),
                community_cards: vec![],
            },
            stacks: vec![],
            pot: None,
            seats: vec![],
            dealer: Some(tf_core::SeatId::new(0)),
            hero: Some(tf_core::SeatId::new(0)),
            timestamp_ms: 0,
        };
        let features = agg.merge(raw);
        assert_eq!(features.street, Street::Preflop);
        assert!(features.hole_cards.is_some());
        assert!(features.community_cards.is_empty());
    }

    #[test]
    fn test_merge_flop() {
        let agg = FeatureAggregator::new("test".into());
        let raw = RawFeatures {
            cards: CardDetectionResult {
                hole_cards: None,
                community_cards: vec![
                    Card { suit: Suit::Spades, rank: Rank::Two, confidence: 0.99 },
                    Card { suit: Suit::Hearts, rank: Rank::Three, confidence: 0.99 },
                    Card { suit: Suit::Diamonds, rank: Rank::Four, confidence: 0.99 },
                ],
            },
            stacks: vec![],
            pot: None,
            seats: vec![],
            dealer: None,
            hero: None,
            timestamp_ms: 0,
        };
        let features = agg.merge(raw);
        assert_eq!(features.street, Street::Flop);
        assert_eq!(features.community_cards.len(), 3);
    }
}
