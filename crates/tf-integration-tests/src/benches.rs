//! 性能基准测试 (基于 criterion 或手动计时)

use std::sync::Arc;
use std::time::Instant;

use tf_core::{Card, Rank, SeatId, Suit, TableEvent};
use tf_rec::{build_rec_input, RecOutput};
use tf_state::TableStateMachine;
use tf_table::TableHandle;
use tf_vision::{CardDetectionResult, FeatureAggregator, PotChange, RawFeatures, StackChange};

use crate::fixtures;

fn bench_state_machine_events(iterations: usize) -> std::time::Duration {
    let mut sm = TableStateMachine::new("bench-sm".to_string());

    let start = Instant::now();
    for i in 0..iterations {
        let _ = sm.process_event(TableEvent::PotChanged {
            new_total: (i as f64) * 10.0,
            delta: 10.0,
        });
    }
    start.elapsed()
}

fn bench_build_rec_input(iterations: usize) -> std::time::Duration {
    let state = fixtures::make_preflop_state();

    let start = Instant::now();
    for _ in 0..iterations {
        let _ = build_rec_input(&state);
    }
    start.elapsed()
}

fn bench_feature_aggregation(iterations: usize) -> std::time::Duration {
    let raw = RawFeatures {
        cards: CardDetectionResult {
            hole_cards: Some([
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
            ]),
            community_cards: vec![
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
        },
        stacks: vec![
            StackChange {
                seat_id: SeatId::new(0),
                prev_estimated: 500.0,
                curr_estimated: 450.0,
                delta: -50.0,
                confidence: 0.8,
            },
            StackChange {
                seat_id: SeatId::new(1),
                prev_estimated: 400.0,
                curr_estimated: 350.0,
                delta: -50.0,
                confidence: 0.75,
            },
        ],
        pot: Some(PotChange {
            prev_value: 0.0,
            new_value: 100.0,
            delta: 100.0,
            timestamp_ms: 1000,
        }),
        seats: vec![],
        dealer: Some(SeatId::new(0)),
        hero: Some(SeatId::new(0)),
        timestamp_ms: 1000,
    };

    let aggregator = FeatureAggregator::new("bench-agg".to_string());

    let start = Instant::now();
    for _ in 0..iterations {
        let _ = aggregator.merge(raw.clone());
    }
    start.elapsed()
}

fn bench_rec_cache_lookup(iterations: usize) -> std::time::Duration {
    use tf_rec::{compute_cache_key, RecCache};

    let state = fixtures::make_preflop_state();
    let input = build_rec_input(&state).unwrap();
    let key = compute_cache_key(&input);

    let cache = RecCache::with_capacity(100);
    let output = RecOutput {
        action: "raise".to_string(),
        amount: 150.0,
        confidence: 0.75,
        distribution: Default::default(),
        ev: 2.5,
        processing_time_ms: 5.0,
    };
    cache.put(key.clone(), output);

    let start = Instant::now();
    for _ in 0..iterations {
        let _ = cache.get(&key);
    }
    start.elapsed()
}

#[tokio::test]
async fn bench_state_machine_throughput() {
    let iterations = 10_000;
    let elapsed = bench_state_machine_events(iterations);
    let per_event_us = elapsed.as_micros() as f64 / iterations as f64;
    assert!(
        per_event_us < 100.0,
        "state machine events should be < 100μs each, got {:.1}μs",
        per_event_us
    );
}

#[tokio::test]
async fn bench_build_rec_input_throughput() {
    let iterations = 10_000;
    let elapsed = bench_build_rec_input(iterations);
    let per_call_us = elapsed.as_micros() as f64 / iterations as f64;
    assert!(
        per_call_us < 50.0,
        "build_rec_input should be < 50μs each, got {:.1}μs",
        per_call_us
    );
}

#[tokio::test]
async fn bench_feature_aggregation_throughput() {
    let iterations = 10_000;
    let elapsed = bench_feature_aggregation(iterations);
    let per_call_us = elapsed.as_micros() as f64 / iterations as f64;
    assert!(
        per_call_us < 50.0,
        "feature aggregation should be < 50μs each, got {:.1}μs",
        per_call_us
    );
}

#[tokio::test]
async fn bench_rec_cache_hit_latency() {
    let iterations = 100_000;
    let elapsed = bench_rec_cache_lookup(iterations);
    let per_lookup_ns = elapsed.as_nanos() as f64 / iterations as f64;
    assert!(
        per_lookup_ns < 1000.0,
        "cache lookup should be < 1μs each, got {:.0}ns",
        per_lookup_ns
    );
}

#[tokio::test]
async fn bench_table_handle_start_stop() {
    let _cal = fixtures::make_calibration();
    let inference_pool = fixtures::make_inference_pool();
    let rec_engine = fixtures::make_rec_engine();

    let iterations = 10;
    let start = Instant::now();
    for i in 0..iterations {
        let handle = TableHandle::start(
            format!("bench-handle-{}", i),
            fixtures::make_calibration(),
            Arc::clone(&inference_pool),
            Arc::clone(&rec_engine),
        )
        .await
        .unwrap();
        handle.shutdown().await.unwrap();
    }
    let elapsed = start.elapsed();
    let per_cycle_ms = elapsed.as_millis() as f64 / iterations as f64;
    assert!(
        per_cycle_ms < 500.0,
        "start+stop should be < 500ms each, got {:.0}ms",
        per_cycle_ms
    );
}

#[tokio::test]
async fn bench_concurrent_rec_requests() {
    let rec_engine = fixtures::make_rec_engine();
    let state = fixtures::make_preflop_state();
    let input = build_rec_input(&state).unwrap();

    let concurrency = 50;
    let start = Instant::now();

    let mut handles = Vec::new();
    for _ in 0..concurrency {
        let eng = Arc::clone(&rec_engine);
        let inp = input.clone();
        handles.push(tokio::spawn(async move { eng.recommend(inp).await }));
    }

    for jh in handles {
        let result = jh.await.unwrap();
        assert!(result.is_ok());
    }

    let elapsed = start.elapsed();
    let per_req_us = elapsed.as_micros() as f64 / concurrency as f64;
    assert!(
        per_req_us < 5000.0,
        "concurrent rec request should avg < 5ms, got {:.0}μs",
        per_req_us
    );
}

#[tokio::test]
async fn bench_single_frame_processing_latency() {
    let mut sm = TableStateMachine::new("bench-frame".to_string());
    sm.process_event(TableEvent::NewHandDetected {
        dealer_seat: SeatId::new(0),
    })
    .unwrap();

    let iterations = 1000;
    let start = Instant::now();

    for _ in 0..iterations {
        let raw = RawFeatures {
            cards: CardDetectionResult {
                hole_cards: Some([
                    Card { suit: Suit::Spades, rank: Rank::Ace, confidence: 0.95 },
                    Card { suit: Suit::Hearts, rank: Rank::King, confidence: 0.94 },
                ]),
                community_cards: vec![],
            },
            stacks: vec![
                StackChange {
                    seat_id: SeatId::new(0),
                    prev_estimated: 500.0,
                    curr_estimated: 500.0,
                    delta: 0.0,
                    confidence: 0.9,
                },
            ],
            pot: Some(PotChange {
                prev_value: 0.0,
                new_value: 30.0,
                delta: 30.0,
                timestamp_ms: 1000,
            }),
            seats: vec![],
            dealer: Some(SeatId::new(0)),
            hero: Some(SeatId::new(0)),
            timestamp_ms: 1000,
        };

        let aggregator = FeatureAggregator::new("bench-frame".to_string());
        let features = aggregator.merge(raw);

        if let Some(cards) = features.hole_cards {
            let _ = sm.process_event(TableEvent::HoleCardsDetected { cards });
        }
        if let Some(ref pc) = features.pot_change {
            let _ = sm.process_event(TableEvent::PotChanged {
                new_total: pc.new_value,
                delta: pc.delta,
            });
        }

        let state = sm.snapshot();
        let _ = build_rec_input(&state);
    }

    let elapsed = start.elapsed();
    let per_frame_us = elapsed.as_micros() as f64 / iterations as f64;
    assert!(
        per_frame_us < 1000.0,
        "single frame processing should be < 1ms (mock), got {:.0}μs",
        per_frame_us
    );
}

#[tokio::test]
async fn bench_end_to_end_recommendation_latency() {
    let engine = fixtures::make_rec_engine();
    let state = fixtures::make_preflop_state();

    let iterations = 1000;
    let start = Instant::now();

    for _ in 0..iterations {
        let input = build_rec_input(&state).unwrap();
        let _ = engine.recommend(input).await.unwrap();
    }

    let elapsed = start.elapsed();
    let per_rec_us = elapsed.as_micros() as f64 / iterations as f64;
    assert!(
        per_rec_us < 500.0,
        "end-to-end recommendation should be < 500μs (mock engine), got {:.0}μs",
        per_rec_us
    );
}
