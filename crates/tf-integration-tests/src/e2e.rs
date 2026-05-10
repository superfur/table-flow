//! 端到端集成测试：State → Rec + State Machine 生命周期 + Vision 聚合

use tf_core::{Card, Rank, SeatId, SeatStatus, Street, Suit, TableEvent};
use tf_rec::{build_rec_input, recommend_from_state, RecOutput};
use tf_state::{BettingRoundEngine, TableStateMachine};
use tf_table::TableHandle;
use tf_vision::{
    CardDetectionResult, FeatureAggregator, PotChange, RawFeatures, SeatChange, StackChange,
};

use crate::fixtures;

#[tokio::test]
async fn test_state_to_recommendation() {
    let state = fixtures::make_preflop_state();
    let engine = fixtures::make_rec_engine();

    let input = build_rec_input(&state);
    assert!(input.is_some(), "build_rec_input should return Some for valid state");

    let rec_input = input.unwrap();
    assert_eq!(rec_input.hole_cards[0].rank, Rank::Ace);
    assert_eq!(rec_input.hole_cards[1].rank, Rank::King);
    assert_eq!(rec_input.num_opponents, 1);
    assert_eq!(rec_input.pot, 100.0);
    assert_eq!(rec_input.stack, 500.0);
    assert_eq!(rec_input.action_history.len(), 2);

    let output = engine.recommend(rec_input).await;
    assert!(output.is_ok(), "recommendation should succeed");

    let rec = output.unwrap();
    assert_eq!(rec.action, "raise");
    assert_eq!(rec.amount, 150.0);
    assert!(rec.confidence > 0.5);
    assert!(rec.distribution.contains_key("raise"));
    assert!(rec.ev > 0.0);
}

#[tokio::test]
async fn test_recommend_from_state_convenience() {
    let state = fixtures::make_preflop_state();
    let engine = fixtures::make_rec_engine();

    let result = recommend_from_state(engine.as_ref(), &state).await;
    assert!(result.is_ok());
    let opt = result.unwrap();
    assert!(opt.is_some());
    let rec = opt.unwrap();
    assert_eq!(rec.action, "raise");
}

#[tokio::test]
async fn test_state_machine_lifecycle() {
    let mut sm = TableStateMachine::new("test-lifecycle".to_string());

    sm.process_event(TableEvent::NewHandDetected {
        dealer_seat: SeatId::new(0),
    })
    .unwrap();
    assert_eq!(sm.state().dealer_seat, Some(SeatId::new(0)));

    sm.process_event(TableEvent::HoleCardsDetected {
        cards: [
            Card {
                suit: Suit::Hearts,
                rank: Rank::Queen,
                confidence: 0.99,
            },
            Card {
                suit: Suit::Diamonds,
                rank: Rank::Jack,
                confidence: 0.98,
            },
        ],
    })
    .unwrap();
    assert!(sm.state().hole_cards.is_some());

    sm.process_event(TableEvent::PotChanged {
        new_total: 75.0,
        delta: 75.0,
    })
    .unwrap();
    assert_eq!(sm.state().pot.total, 75.0);

    sm.process_event(TableEvent::DealerButtonMoved {
        new_seat: SeatId::new(3),
    })
    .unwrap();
    assert_eq!(sm.state().dealer_seat, Some(SeatId::new(3)));
}

#[tokio::test]
async fn test_state_machine_street_transitions() {
    let mut sm = TableStateMachine::new("street-test".to_string());

    sm.process_event(TableEvent::NewHandDetected {
        dealer_seat: SeatId::new(0),
    })
    .unwrap();
    assert_eq!(sm.state().street, Street::Preflop);

    sm.process_event(TableEvent::HoleCardsDetected {
        cards: [
            Card {
                suit: Suit::Spades,
                rank: Rank::Ace,
                confidence: 0.95,
            },
            Card {
                suit: Suit::Hearts,
                rank: Rank::King,
                confidence: 0.94,
            },
        ],
    })
    .unwrap();

    sm.process_event(TableEvent::CommunityCardsChanged {
        cards: vec![
            Card {
                suit: Suit::Hearts,
                rank: Rank::Two,
                confidence: 0.9,
            },
            Card {
                suit: Suit::Diamonds,
                rank: Rank::Five,
                confidence: 0.9,
            },
            Card {
                suit: Suit::Clubs,
                rank: Rank::Eight,
                confidence: 0.9,
            },
        ],
        street: Street::Flop,
    })
    .unwrap();
    assert_eq!(sm.state().street, Street::Flop);
    assert_eq!(sm.state().community_cards.len(), 3);

    sm.process_event(TableEvent::CommunityCardsChanged {
        cards: vec![
            Card {
                suit: Suit::Hearts,
                rank: Rank::Two,
                confidence: 0.9,
            },
            Card {
                suit: Suit::Diamonds,
                rank: Rank::Five,
                confidence: 0.9,
            },
            Card {
                suit: Suit::Clubs,
                rank: Rank::Eight,
                confidence: 0.9,
            },
            Card {
                suit: Suit::Spades,
                rank: Rank::Ten,
                confidence: 0.9,
            },
        ],
        street: Street::Turn,
    })
    .unwrap();
    assert_eq!(sm.state().street, Street::Turn);
    assert_eq!(sm.state().community_cards.len(), 4);
}

#[tokio::test]
async fn test_vision_aggregation_to_state() {
    let raw = RawFeatures {
        cards: CardDetectionResult {
            hole_cards: Some([
                Card {
                    suit: Suit::Clubs,
                    rank: Rank::Ten,
                    confidence: 0.9,
                },
                Card {
                    suit: Suit::Spades,
                    rank: Rank::Nine,
                    confidence: 0.88,
                },
            ]),
            community_cards: vec![
                Card {
                    suit: Suit::Hearts,
                    rank: Rank::Two,
                    confidence: 0.95,
                },
                Card {
                    suit: Suit::Diamonds,
                    rank: Rank::Five,
                    confidence: 0.93,
                },
                Card {
                    suit: Suit::Clubs,
                    rank: Rank::Eight,
                    confidence: 0.91,
                },
            ],
        },
        stacks: vec![StackChange {
            seat_id: SeatId::new(0),
            prev_estimated: 500.0,
            curr_estimated: 450.0,
            delta: -50.0,
            confidence: 0.8,
        }],
        pot: Some(PotChange {
            prev_value: 0.0,
            new_value: 150.0,
            delta: 150.0,
            timestamp_ms: 2000,
        }),
        seats: vec![SeatChange {
            seat_id: SeatId::new(1),
            prev_status: Some(SeatStatus::Active),
            new_status: SeatStatus::Folded,
        }],
        dealer: Some(SeatId::new(0)),
        hero: Some(SeatId::new(0)),
        timestamp_ms: 2000,
    };

    let aggregator = FeatureAggregator::new("e2e-agg-table".to_string());
    let features = aggregator.merge(raw);

    assert!(features.hole_cards.is_some());
    assert_eq!(features.community_cards.len(), 3);
    assert_eq!(features.street, Street::Flop);
    assert_eq!(features.dealer_seat, Some(SeatId::new(0)));
    assert_eq!(features.hero_seat, Some(SeatId::new(0)));
    assert!(!features.stack_changes.is_empty());
    assert!(features.pot_change.is_some());
}

#[tokio::test]
async fn test_vision_aggregation_street_inference() {
    let cases = vec![
        (0usize, Street::Preflop),
        (3, Street::Flop),
        (4, Street::Turn),
        (5, Street::River),
    ];

    for (num_community, expected_street) in cases {
        let raw = RawFeatures {
            cards: CardDetectionResult {
                hole_cards: Some([
                    Card {
                        suit: Suit::Spades,
                        rank: Rank::Ace,
                        confidence: 0.9,
                    },
                    Card {
                        suit: Suit::Hearts,
                        rank: Rank::King,
                        confidence: 0.9,
                    },
                ]),
                community_cards: (0..num_community)
                    .map(|i| Card {
                        suit: Suit::Hearts,
                        rank: Rank::Two,
                        confidence: 0.9 + i as f32 * 0.01,
                    })
                    .collect(),
            },
            stacks: vec![],
            pot: None,
            seats: vec![],
            dealer: None,
            hero: Some(SeatId::new(0)),
            timestamp_ms: 1000,
        };

        let aggregator = FeatureAggregator::new("street-test".to_string());
        let features = aggregator.merge(raw);
        assert_eq!(
            features.street, expected_street,
            "expected {:?} with {} community cards",
            expected_street, num_community
        );
    }
}

#[tokio::test]
async fn test_betting_round_logic() {
    let state = fixtures::make_preflop_state();

    let is_complete = BettingRoundEngine::is_round_complete(&state);
    assert!(
        !is_complete,
        "round should not be complete — hero has not acted"
    );

    let to_call = BettingRoundEngine::to_call_for(&state, SeatId::new(0));
    assert_eq!(to_call, 50.0, "hero current_bet is 0, max_bet is 50");

    let min_raise = BettingRoundEngine::min_raise(&state);
    assert!(min_raise >= 50.0, "min raise should be at least BB");
}

#[tokio::test]
async fn test_no_hero_no_recommendation() {
    let mut state = fixtures::make_preflop_state();
    state.hero_seat = None;

    let input = build_rec_input(&state);
    assert!(input.is_none(), "should return None when no hero seat");
}

#[tokio::test]
async fn test_no_hole_cards_no_recommendation() {
    let mut state = fixtures::make_preflop_state();
    state.hole_cards = None;

    let input = build_rec_input(&state);
    assert!(input.is_none(), "should return None when no hole cards");
}

#[tokio::test]
async fn test_full_table_handle_lifecycle() {
    let cal = fixtures::make_calibration();
    let inference_pool = fixtures::make_inference_pool();
    let rec_engine = fixtures::make_rec_engine();

    let handle = TableHandle::start(
        "e2e-table-1".to_string(),
        cal,
        inference_pool,
        rec_engine,
    )
    .await
    .unwrap();

    let sm_arc = handle.state_machine();
    let sm = sm_arc.lock();
    assert_eq!(sm.state().table_id, "e2e-table-1");
    drop(sm);

    let token = handle.cancel_token();
    assert!(!token.is_cancelled());

    handle.shutdown().await.unwrap();
    assert!(token.is_cancelled());
}

#[tokio::test]
async fn test_table_handle_state_machine_interaction() {
    let cal = fixtures::make_calibration();
    let inference_pool = fixtures::make_inference_pool();
    let rec_engine = fixtures::make_rec_engine();

    let handle = TableHandle::start(
        "e2e-sm-test".to_string(),
        cal,
        inference_pool,
        rec_engine,
    )
    .await
    .unwrap();

    {
        let sm_arc = handle.state_machine();
        let mut sm = sm_arc.lock();
        sm.process_event(TableEvent::NewHandDetected {
            dealer_seat: SeatId::new(0),
        })
        .unwrap();
        assert_eq!(sm.state().dealer_seat, Some(SeatId::new(0)));

        sm.process_event(TableEvent::HoleCardsDetected {
            cards: [
                Card {
                    suit: Suit::Spades,
                    rank: Rank::Ace,
                    confidence: 0.95,
                },
                Card {
                    suit: Suit::Hearts,
                    rank: Rank::King,
                    confidence: 0.94,
                },
            ],
        })
        .unwrap();
        assert!(sm.state().hole_cards.is_some());
    }

    handle.shutdown().await.unwrap();
}

#[tokio::test]
async fn test_multiple_state_events_sequence() {
    let mut sm = TableStateMachine::new("seq-test".to_string());

    sm.process_event(TableEvent::NewHandDetected {
        dealer_seat: SeatId::new(2),
    })
    .unwrap();

    sm.process_event(TableEvent::SeatStatusChanged {
        seat_id: SeatId::new(1),
        new_status: SeatStatus::Folded,
    })
    .unwrap();

    sm.process_event(TableEvent::PotChanged {
        new_total: 75.0,
        delta: 25.0,
    })
    .unwrap();

    sm.process_event(TableEvent::PotChanged {
        new_total: 125.0,
        delta: 50.0,
    })
    .unwrap();

    assert_eq!(sm.state().pot.total, 125.0);

    let snapshot = sm.snapshot();
    assert_eq!(snapshot.table_id, "seq-test");
}

#[tokio::test]
async fn test_mock_rec_engine_health() {
    let engine = fixtures::make_rec_engine();
    let health = engine.health().await;
    assert!(health.is_ok());
}

#[tokio::test]
async fn test_rec_cache_integration() {
    use tf_rec::{compute_cache_key, RecCache};

    let cache = RecCache::with_capacity(3);

    let state = fixtures::make_preflop_state();
    let input = build_rec_input(&state).unwrap();
    let key = compute_cache_key(&input);

    assert!(cache.get(&key).is_none());

    let output = RecOutput {
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
        processing_time_ms: 5.0,
    };

    cache.put(key.clone(), output);
    let cached = cache.get(&key);
    assert!(cached.is_some());
    assert_eq!(cached.unwrap().action, "raise");
}

#[tokio::test]
async fn test_state_validator_integration() {
    use tf_state::StateValidator;

    let state = fixtures::make_preflop_state();
    let result = StateValidator::validate(&state);
    match result {
        tf_state::ValidationResult::Valid => {},
        tf_state::ValidationResult::Issues(issues) => {
            panic!("valid state should not have issues: {:?}", issues);
        }
    }
}
