//! Sidecar integration tests: state → build_rec_input → RecSidecar → RecOutput

use std::time::Duration;

use tf_core::{Card, Rank, Street, Suit};
use tf_rec::{RecEngine, RecInput, RecSidecar, SidecarConfig};

use crate::fixtures;

fn sidecar_config() -> SidecarConfig {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".to_string());
    let script = std::path::PathBuf::from(&manifest_dir)
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("rec-sidecar/index.js");

    SidecarConfig {
        script_path: script,
        request_timeout: Duration::from_secs(5),
        ..Default::default()
    }
}

fn make_aa_preflop_input() -> RecInput {
    RecInput {
        hole_cards: [
            Card { suit: Suit::Spades, rank: Rank::Ace, confidence: 0.99 },
            Card { suit: Suit::Spades, rank: Rank::Ace, confidence: 0.98 },
        ],
        community_cards: vec![],
        pot: 100.0,
        to_call: 50.0,
        min_raise: 100.0,
        stack: 500.0,
        street: Street::Preflop,
        num_opponents: 2,
        action_history: vec![],
    }
}

fn make_weak_preflop_input() -> RecInput {
    RecInput {
        hole_cards: [
            Card { suit: Suit::Clubs, rank: Rank::Two, confidence: 0.95 },
            Card { suit: Suit::Diamonds, rank: Rank::Seven, confidence: 0.93 },
        ],
        community_cards: vec![],
        pot: 80.0,
        to_call: 40.0,
        min_raise: 80.0,
        stack: 300.0,
        street: Street::Preflop,
        num_opponents: 3,
        action_history: vec![],
    }
}

fn make_ak_flop_input() -> RecInput {
    RecInput {
        hole_cards: [
            Card { suit: Suit::Hearts, rank: Rank::Ace, confidence: 0.97 },
            Card { suit: Suit::Hearts, rank: Rank::King, confidence: 0.96 },
        ],
        community_cards: vec![
            Card { suit: Suit::Hearts, rank: Rank::Two, confidence: 0.9 },
            Card { suit: Suit::Diamonds, rank: Rank::Five, confidence: 0.9 },
            Card { suit: Suit::Clubs, rank: Rank::Eight, confidence: 0.9 },
        ],
        pot: 200.0,
        to_call: 0.0,
        min_raise: 100.0,
        stack: 500.0,
        street: Street::Flop,
        num_opponents: 2,
        action_history: vec![],
    }
}

macro_rules! sidecar_skip_if_no_script {
    ($config:expr) => {
        if !$config.script_path.exists() {
            eprintln!(
                "Skipping sidecar e2e test: script not found at {:?}",
                $config.script_path
            );
            return;
        }
    };
}

#[tokio::test]
async fn test_sidecar_health() {
    let config = sidecar_config();
    sidecar_skip_if_no_script!(config);

    let sidecar = RecSidecar::spawn(config).await.unwrap();
    sidecar.health().await.unwrap();
    sidecar.shutdown().await.unwrap();
}

#[tokio::test]
async fn test_sidecar_aa_preflop_raises() {
    let config = sidecar_config();
    sidecar_skip_if_no_script!(config);

    let sidecar = RecSidecar::spawn(config).await.unwrap();

    let output = sidecar.recommend(make_aa_preflop_input()).await.unwrap();
    assert_eq!(output.action, "raise", "AA preflop should recommend raise");
    assert!(output.amount > 0.0, "raise should have positive amount");
    assert!(output.confidence > 0.5, "AA should have high confidence");
    assert!(
        output.distribution.contains_key("raise"),
        "distribution should include raise"
    );

    sidecar.shutdown().await.unwrap();
}

#[tokio::test]
async fn test_sidecar_weak_hand_folds() {
    let config = sidecar_config();
    sidecar_skip_if_no_script!(config);

    let sidecar = RecSidecar::spawn(config).await.unwrap();

    let output = sidecar.recommend(make_weak_preflop_input()).await.unwrap();
    assert!(
        output.action == "fold",
        "72o preflop with bet should recommend fold, got: {}",
        output.action
    );

    sidecar.shutdown().await.unwrap();
}

#[tokio::test]
async fn test_sidecar_ak_flop_strength() {
    let config = sidecar_config();
    sidecar_skip_if_no_script!(config);

    let sidecar = RecSidecar::spawn(config).await.unwrap();

    let output = sidecar.recommend(make_ak_flop_input()).await.unwrap();
    assert!(
        matches!(output.action.as_str(), "raise" | "check" | "call"),
        "AK on flop should be a reasonable action, got: {}",
        output.action
    );
    assert!(output.confidence > 0.3);

    sidecar.shutdown().await.unwrap();
}

#[tokio::test]
async fn test_sidecar_multiple_requests() {
    let config = sidecar_config();
    sidecar_skip_if_no_script!(config);

    let sidecar = RecSidecar::spawn(config).await.unwrap();

    let out1 = sidecar.recommend(make_aa_preflop_input()).await.unwrap();
    assert_eq!(out1.action, "raise");

    let out2 = sidecar.recommend(make_weak_preflop_input()).await.unwrap();
    assert_eq!(out2.action, "fold");

    let out3 = sidecar.recommend(make_ak_flop_input()).await.unwrap();
    assert!(!out3.action.is_empty());

    sidecar.shutdown().await.unwrap();
}

#[tokio::test]
async fn test_sidecar_build_rec_input_integration() {
    let config = sidecar_config();
    sidecar_skip_if_no_script!(config);

    let sidecar = RecSidecar::spawn(config).await.unwrap();

    let state = fixtures::make_preflop_state();
    let input = tf_rec::build_rec_input(&state);
    assert!(input.is_some());

    let output = sidecar.recommend(input.unwrap()).await.unwrap();
    assert!(!output.action.is_empty());
    assert!(output.confidence > 0.0);

    sidecar.shutdown().await.unwrap();
}

#[tokio::test]
async fn test_sidecar_restart() {
    let config = sidecar_config();
    sidecar_skip_if_no_script!(config);

    let sidecar = RecSidecar::spawn(config).await.unwrap();
    sidecar.health().await.unwrap();

    sidecar.restart().await.unwrap();
    sidecar.health().await.unwrap();

    let output = sidecar.recommend(make_aa_preflop_input()).await.unwrap();
    assert_eq!(output.action, "raise");

    sidecar.shutdown().await.unwrap();
}

#[tokio::test]
async fn test_sidecar_hand_ranking_order() {
    let config = sidecar_config();
    sidecar_skip_if_no_script!(config);

    let sidecar = RecSidecar::spawn(config).await.unwrap();

    let aa = sidecar.recommend(make_aa_preflop_input()).await.unwrap();
    let weak = sidecar.recommend(make_weak_preflop_input()).await.unwrap();

    assert!(
        aa.confidence > weak.confidence,
        "AA confidence ({}) should be higher than 72o ({})",
        aa.confidence,
        weak.confidence
    );

    sidecar.shutdown().await.unwrap();
}
