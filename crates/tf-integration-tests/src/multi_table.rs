//! 多桌并发集成测试

use std::sync::Arc;

use tf_core::{
    CaptureBackend, Card, InferenceConfig, ManagerConfig, Rank, SeatId, Suit, TableCalibration,
    TableEvent, ThreadConfig,
};
use tf_rec::{MockRecEngine, RecEngine, RecOutput};
use tf_table::TableManager;
use tf_state::TableStateMachine;

use crate::fixtures;

fn make_unique_calibration(table_idx: usize) -> TableCalibration {
    let mut cal = fixtures::make_calibration();
    cal.theme_id = format!("test-table-{}", table_idx);
    cal
}

fn make_manager_config() -> ManagerConfig {
    ManagerConfig {
        max_tables: 8,
        fps_per_table: 10,
        capture_backend: CaptureBackend::Dxgi,
        thread_config: ThreadConfig::default(),
        inference_config: InferenceConfig {
            card_model_path: "/tmp/card.onnx".into(),
            digit_model_path: "/tmp/digit.onnx".into(),
            onnx_intra_threads: 1,
            onnx_inter_threads: 1,
            card_session_count: 1,
            digit_session_count: 1,
            use_gpu: false,
        },
    }
}

fn make_rec_engine_for_table(table_idx: usize) -> Arc<dyn RecEngine> {
    Arc::new(MockRecEngine::new(RecOutput {
        action: format!("action-{}", table_idx),
        amount: 100.0 + table_idx as f64 * 10.0,
        confidence: 0.8,
        distribution: {
            let mut m = std::collections::HashMap::new();
            m.insert("fold".to_string(), 0.2);
            m.insert("raise".to_string(), 0.8);
            m
        },
        ev: 1.5 + table_idx as f64,
        processing_time_ms: 5.0,
    }))
}

#[tokio::test]
async fn test_start_multiple_tables_concurrently() {
    let _cal = fixtures::make_calibration();
    let inference_pool = fixtures::make_inference_pool();
    let rec_engine = fixtures::make_rec_engine();

    let num_tables = 4;
    let mut handles = Vec::new();

    for i in 0..num_tables {
        let table_id = format!("multi-table-{}", i);
        let handle = tf_table::TableHandle::start(
            table_id,
            make_unique_calibration(i),
            Arc::clone(&inference_pool),
            Arc::clone(&rec_engine),
        )
        .await
        .unwrap();
        handles.push(handle);
    }

    for (i, handle) in handles.iter().enumerate() {
        let sm = handle.state_machine();
        let state = sm.lock();
        assert_eq!(state.state().table_id, format!("multi-table-{}", i));
    }

    for handle in handles {
        handle.shutdown().await.unwrap();
    }
}

#[tokio::test]
async fn test_concurrent_state_machine_updates() {
    let num_tables = 4;
    let state_machines: Vec<Arc<parking_lot::Mutex<TableStateMachine>>> = (0..num_tables)
        .map(|i| Arc::new(parking_lot::Mutex::new(TableStateMachine::new(format!("csm-{}", i)))))
        .collect();

    let mut join_handles = Vec::new();

    for (idx, sm_arc) in state_machines.into_iter().enumerate() {
        let handle = tokio::spawn(async move {
            let mut sm = sm_arc.lock();
            sm.process_event(TableEvent::NewHandDetected {
                dealer_seat: SeatId::new(idx as u8),
            })
            .unwrap();

            sm.process_event(TableEvent::HoleCardsDetected {
                cards: [
                    Card {
                        suit: if idx % 2 == 0 { Suit::Spades } else { Suit::Hearts },
                        rank: Rank::Ace,
                        confidence: 0.95,
                    },
                    Card {
                        suit: if idx % 2 == 0 { Suit::Hearts } else { Suit::Diamonds },
                        rank: Rank::King,
                        confidence: 0.94,
                    },
                ],
            })
            .unwrap();

            sm.process_event(TableEvent::PotChanged {
                new_total: 50.0 * (idx + 1) as f64,
                delta: 50.0,
            })
            .unwrap();

            let state = sm.state();
            let table_id = state.table_id.clone();
            let dealer = state.dealer_seat;
            let pot = state.pot.total;
            (table_id, dealer, pot)
        });
        join_handles.push(handle);
    }

    for (i, jh) in join_handles.into_iter().enumerate() {
        let (table_id, dealer, pot) = jh.await.unwrap();
        assert_eq!(table_id, format!("csm-{}", i));
        assert_eq!(dealer, Some(SeatId::new(i as u8)));
        assert_eq!(pot, 50.0 * (i + 1) as f64);
    }
}

#[tokio::test]
async fn test_independent_shutdown_order() {
    let inference_pool = fixtures::make_inference_pool();
    let rec_engine = fixtures::make_rec_engine();

    let h0 = tf_table::TableHandle::start(
        "order-0".to_string(),
        make_unique_calibration(0),
        Arc::clone(&inference_pool),
        Arc::clone(&rec_engine),
    )
    .await
    .unwrap();

    let h1 = tf_table::TableHandle::start(
        "order-1".to_string(),
        make_unique_calibration(1),
        Arc::clone(&inference_pool),
        Arc::clone(&rec_engine),
    )
    .await
    .unwrap();

    let h2 = tf_table::TableHandle::start(
        "order-2".to_string(),
        make_unique_calibration(2),
        Arc::clone(&inference_pool),
        Arc::clone(&rec_engine),
    )
    .await
    .unwrap();

    let t0 = h0.cancel_token();
    let t1 = h1.cancel_token();
    let t2 = h2.cancel_token();

    assert!(!t0.is_cancelled());
    assert!(!t1.is_cancelled());
    assert!(!t2.is_cancelled());

    h1.shutdown().await.unwrap();
    assert!(!t0.is_cancelled());
    assert!(t1.is_cancelled());
    assert!(!t2.is_cancelled());

    h0.shutdown().await.unwrap();
    assert!(t0.is_cancelled());
    assert!(t1.is_cancelled());
    assert!(!t2.is_cancelled());

    h2.shutdown().await.unwrap();
    assert!(t2.is_cancelled());
}

#[tokio::test]
async fn test_table_manager_lifecycle() {
    let config = make_manager_config();
    let rec_engine = make_rec_engine_for_table(0);
    let manager = TableManager::with_rec_engine(config, rec_engine).unwrap();

    assert!(manager.list_tables().is_empty());

    let cal0 = make_unique_calibration(0);
    let cal1 = make_unique_calibration(1);

    manager
        .start_table("mgr-table-0".to_string(), cal0)
        .await
        .unwrap();
    manager
        .start_table("mgr-table-1".to_string(), cal1)
        .await
        .unwrap();

    let tables = manager.list_tables();
    assert_eq!(tables.len(), 2);

    manager.stop_table(&"mgr-table-0".to_string()).await.unwrap();
    let tables = manager.list_tables();
    assert_eq!(tables.len(), 1);
    assert!(tables.contains(&"mgr-table-1".to_string()));

    manager.shutdown_all().await.unwrap();
    assert!(manager.list_tables().is_empty());
}

#[tokio::test]
async fn test_table_manager_multiple_starts() {
    let mut config = make_manager_config();
    config.max_tables = 8;

    let rec_engine = make_rec_engine_for_table(0);
    let manager = TableManager::with_rec_engine(config, rec_engine).unwrap();

    manager
        .start_table("multi-0".to_string(), make_unique_calibration(0))
        .await
        .unwrap();
    manager
        .start_table("multi-1".to_string(), make_unique_calibration(1))
        .await
        .unwrap();

    let result = manager
        .start_table("multi-0".to_string(), make_unique_calibration(0))
        .await;
    assert!(result.is_err(), "duplicate table_id should fail");

    manager.shutdown_all().await.unwrap();
}

#[tokio::test]
async fn test_parallel_rec_requests() {
    let rec_engine = fixtures::make_rec_engine();
    let state = fixtures::make_preflop_state();

    let input = tf_rec::build_rec_input(&state).unwrap();

    let mut handles = Vec::new();
    for _ in 0..8 {
        let eng = Arc::clone(&rec_engine);
        let inp = input.clone();
        handles.push(tokio::spawn(async move { eng.recommend(inp).await }));
    }

    for jh in handles {
        let result = jh.await.unwrap();
        assert!(result.is_ok());
        assert_eq!(result.unwrap().action, "raise");
    }
}

#[tokio::test]
async fn test_cancel_token_clone_independence() {
    let inference_pool = fixtures::make_inference_pool();
    let rec_engine = fixtures::make_rec_engine();

    let handle = tf_table::TableHandle::start(
        "cancel-test".to_string(),
        fixtures::make_calibration(),
        inference_pool,
        rec_engine,
    )
    .await
    .unwrap();

    let token1 = handle.cancel_token();
    let token2 = token1.clone();

    assert!(!token1.is_cancelled());
    assert!(!token2.is_cancelled());

    token1.cancel();

    assert!(token1.is_cancelled());
    assert!(token2.is_cancelled(), "cloned token should reflect cancellation");

    handle.shutdown().await.unwrap();
}

#[tokio::test]
async fn test_8_table_concurrent_lifecycle() {
    let inference_pool = fixtures::make_inference_pool();
    let rec_engine = fixtures::make_rec_engine();

    let num_tables = 8;
    let mut handles = Vec::new();

    let start = std::time::Instant::now();
    for i in 0..num_tables {
        let handle = tf_table::TableHandle::start(
            format!("8table-{}", i),
            make_unique_calibration(i),
            Arc::clone(&inference_pool),
            Arc::clone(&rec_engine),
        )
        .await
        .unwrap();
        handles.push(handle);
    }
    let spawn_elapsed = start.elapsed();

    for (i, handle) in handles.iter().enumerate() {
        let sm = handle.state_machine();
        let state = sm.lock();
        assert_eq!(state.state().table_id, format!("8table-{}", i));
    }

    let start = std::time::Instant::now();
    for handle in handles {
        handle.shutdown().await.unwrap();
    }
    let shutdown_elapsed = start.elapsed();

    assert!(
        spawn_elapsed.as_millis() < 5000,
        "8 table spawn should be < 5s, got {}ms",
        spawn_elapsed.as_millis()
    );
    assert!(
        shutdown_elapsed.as_millis() < 3000,
        "8 table shutdown should be < 3s, got {}ms",
        shutdown_elapsed.as_millis()
    );
}

#[tokio::test]
async fn test_8_table_concurrent_state_updates() {
    let num_tables = 8;
    let sms: Vec<Arc<parking_lot::Mutex<TableStateMachine>>> = (0..num_tables)
        .map(|i| Arc::new(parking_lot::Mutex::new(TableStateMachine::new(format!("c8-{}", i)))))
        .collect();

    let mut join_handles = Vec::new();

    for (idx, sm_arc) in sms.into_iter().enumerate() {
        let handle = tokio::task::spawn_blocking(move || {
            let mut sm = sm_arc.lock();
            sm.process_event(TableEvent::NewHandDetected {
                dealer_seat: SeatId::new(idx as u8),
            })
            .unwrap();
            sm.process_event(TableEvent::HoleCardsDetected {
                cards: [
                    Card { suit: Suit::Spades, rank: Rank::Ace, confidence: 0.95 },
                    Card { suit: Suit::Hearts, rank: Rank::King, confidence: 0.94 },
                ],
            })
            .unwrap();
            sm.process_event(TableEvent::PotChanged { new_total: 100.0, delta: 100.0 }).unwrap();
            sm.process_event(TableEvent::CommunityCardsChanged {
                cards: vec![
                    Card { suit: Suit::Hearts, rank: Rank::Two, confidence: 0.9 },
                    Card { suit: Suit::Diamonds, rank: Rank::Five, confidence: 0.9 },
                    Card { suit: Suit::Clubs, rank: Rank::Eight, confidence: 0.9 },
                ],
                street: tf_core::Street::Flop,
            })
            .unwrap();
            let state = sm.snapshot();
            (state.table_id.clone(), state.street, state.pot.total)
        });
        join_handles.push(handle);
    }

    for (i, jh) in join_handles.into_iter().enumerate() {
        let (table_id, street, pot) = jh.await.unwrap();
        assert_eq!(table_id, format!("c8-{}", i));
        assert_eq!(street, tf_core::Street::Flop);
        assert!((pot - 100.0).abs() < 0.01);
    }
}

#[tokio::test]
async fn test_8_table_no_deadlock() {
    let inference_pool = fixtures::make_inference_pool();
    let rec_engine = fixtures::make_rec_engine();

    let num_tables = 8;
    let mut handles = Vec::new();

    for i in 0..num_tables {
        let handle = tf_table::TableHandle::start(
            format!("deadlock-{}", i),
            make_unique_calibration(i),
            Arc::clone(&inference_pool),
            Arc::clone(&rec_engine),
        )
        .await
        .unwrap();
        handles.push(handle);
    }

    let result = tokio::time::timeout(
        std::time::Duration::from_secs(5),
        async {
            for handle in handles {
                handle.shutdown().await.unwrap();
            }
        },
    )
    .await;

    assert!(result.is_ok(), "8 table shutdown should complete within 5s (no deadlock)");
}
