//! 测试 fixture 工具函数

use std::sync::Arc;

use tf_core::{
    BlindsInfo, Card, DigitOcrRegions, InferenceConfig, NormalizedRect, Rank, SeatCalibration,
    SeatId, SeatStatus, Street, Suit, TableCalibration,
};
use tf_inference::InferencePool;
use tf_rec::{MockRecEngine, RecEngine, RecOutput};
use tf_state::{ActionRecord, PotInfo, SeatState, TableState};

pub fn make_calibration() -> TableCalibration {
    TableCalibration {
        resolution: (1920, 1080),
        hole_card_positions: [
            NormalizedRect::new(0.1, 0.5, 0.05, 0.08),
            NormalizedRect::new(0.17, 0.5, 0.05, 0.08),
        ],
        community_card_positions: [
            NormalizedRect::new(0.35, 0.4, 0.05, 0.08),
            NormalizedRect::new(0.42, 0.4, 0.05, 0.08),
            NormalizedRect::new(0.49, 0.4, 0.05, 0.08),
            NormalizedRect::new(0.56, 0.4, 0.05, 0.08),
            NormalizedRect::new(0.63, 0.4, 0.05, 0.08),
        ],
        pot_position: NormalizedRect::new(0.45, 0.3, 0.1, 0.04),
        seat_positions: vec![
            SeatCalibration {
                seat_id: SeatId::new(0),
                seat_region: NormalizedRect::new(0.0, 0.0, 0.1, 0.1),
                stack_region: NormalizedRect::new(0.0, 0.08, 0.1, 0.03),
                bet_region: NormalizedRect::new(0.05, 0.12, 0.05, 0.03),
                avatar_region: NormalizedRect::new(0.0, 0.0, 0.05, 0.05),
                card_region: Some(NormalizedRect::new(0.02, 0.0, 0.04, 0.06)),
            },
            SeatCalibration {
                seat_id: SeatId::new(1),
                seat_region: NormalizedRect::new(0.4, 0.0, 0.1, 0.1),
                stack_region: NormalizedRect::new(0.4, 0.08, 0.1, 0.03),
                bet_region: NormalizedRect::new(0.45, 0.12, 0.05, 0.03),
                avatar_region: NormalizedRect::new(0.4, 0.0, 0.05, 0.05),
                card_region: None,
            },
            SeatCalibration {
                seat_id: SeatId::new(2),
                seat_region: NormalizedRect::new(0.8, 0.0, 0.1, 0.1),
                stack_region: NormalizedRect::new(0.8, 0.08, 0.1, 0.03),
                bet_region: NormalizedRect::new(0.85, 0.12, 0.05, 0.03),
                avatar_region: NormalizedRect::new(0.8, 0.0, 0.05, 0.05),
                card_region: None,
            },
        ],
        dealer_button_region: NormalizedRect::new(0.3, 0.35, 0.03, 0.03),
        action_button_regions: [
            NormalizedRect::new(0.4, 0.85, 0.08, 0.04),
            NormalizedRect::new(0.5, 0.85, 0.08, 0.04),
            NormalizedRect::new(0.6, 0.85, 0.08, 0.04),
            NormalizedRect::new(0.7, 0.85, 0.08, 0.04),
        ],
        hero_seat: Some(SeatId::new(0)),
        blinds: BlindsInfo {
            small_blind: 25.0,
            big_blind: 50.0,
            ante: 0.0,
            straddle: 0.0,
            current_max_bet: 50.0,
            last_raise_size: 50.0,
        },
        digit_ocr_regions: DigitOcrRegions::default(),
        theme_id: "test".to_string(),
    }
}

pub fn make_preflop_state() -> TableState {
    TableState {
        table_id: "e2e-test-table".to_string(),
        phase: tf_core::TablePhase::Playing,
        street: Street::Preflop,
        hand_number: 1,
        dealer_seat: Some(SeatId::new(1)),
        hero_seat: Some(SeatId::new(0)),
        hole_cards: Some([
            Card { suit: Suit::Spades, rank: Rank::Ace, confidence: 0.98 },
            Card { suit: Suit::Hearts, rank: Rank::King, confidence: 0.97 },
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
                action: tf_core::ActionType::PostBlind(tf_core::BlindKind::SmallBlind),
                amount: 25.0,
                street: Street::Preflop,
                seq: 0,
                confidence: 1.0,
            },
            ActionRecord {
                seat_id: SeatId::new(0),
                action: tf_core::ActionType::PostBlind(tf_core::BlindKind::BigBlind),
                amount: 50.0,
                street: Street::Preflop,
                seq: 1,
                confidence: 1.0,
            },
            ActionRecord {
                seat_id: SeatId::new(2),
                action: tf_core::ActionType::Fold,
                amount: 0.0,
                street: Street::Preflop,
                seq: 2,
                confidence: 0.95,
            },
            ActionRecord {
                seat_id: SeatId::new(1),
                action: tf_core::ActionType::Call,
                amount: 25.0,
                street: Street::Preflop,
                seq: 3,
                confidence: 0.92,
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
        last_update_ms: 1000,
        state_confidence: 0.95,
    }
}

pub fn make_flop_state() -> TableState {
    let mut state = make_preflop_state();
    state.street = Street::Flop;
    state.community_cards = vec![
        Card { suit: Suit::Hearts, rank: Rank::Two, confidence: 0.95 },
        Card { suit: Suit::Diamonds, rank: Rank::Five, confidence: 0.93 },
        Card { suit: Suit::Clubs, rank: Rank::Eight, confidence: 0.91 },
    ];
    state.blinds.current_max_bet = 0.0;
    state.last_update_ms = 3000;
    state
}

pub fn make_rec_engine() -> Arc<dyn RecEngine> {
    Arc::new(MockRecEngine::new(RecOutput {
        action: "raise".to_string(),
        amount: 150.0,
        confidence: 0.75,
        distribution: {
            let mut m = std::collections::HashMap::new();
            m.insert("fold".to_string(), 0.1);
            m.insert("call".to_string(), 0.15);
            m.insert("raise".to_string(), 0.75);
            m
        },
        ev: 2.5,
        processing_time_ms: 8.0,
    }))
}

pub fn make_inference_pool() -> Arc<InferencePool> {
    Arc::new(
        InferencePool::new(InferenceConfig {
            card_model_path: "/tmp/card.onnx".into(),
            digit_model_path: "/tmp/digit.onnx".into(),
            onnx_intra_threads: 1,
            onnx_inter_threads: 1,
            card_session_count: 1,
            digit_session_count: 1,
            use_gpu: false,
        })
        .unwrap(),
    )
}
