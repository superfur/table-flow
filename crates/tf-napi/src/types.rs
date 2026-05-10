//! JS↔Rust 类型 marshalling。
//!
//! 这一层负责把 Rust 内部类型（含 `f64 NaN`、`Instant`、自定义 enum 等）
//! 转换为 JS-friendly 的扁平结构。

use serde::Serialize;

use tf_core::{ActionType, BlindKind, Card, SeatId, SeatStatus, Street, TablePhase};

#[derive(Debug, Clone, Serialize)]
pub struct JsCard {
    pub suit: String,
    pub rank: String,
}

impl From<Card> for JsCard {
    fn from(c: Card) -> Self {
        Self {
            suit: c.suit.short_str().to_string(),
            rank: c.rank.short_str().to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JsSeat {
    pub seat_id: u8,
    pub status: String,
    pub stack: f64,
    pub current_bet: f64,
    pub last_action: Option<String>,
    pub is_hero: bool,
    pub has_cards: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JsTableState {
    pub table_id: String,
    pub phase: String,
    pub street: String,
    pub hand_number: u64,
    pub dealer_seat: Option<u8>,
    pub hero_seat: Option<u8>,
    pub hole_cards: Option<Vec<JsCard>>,
    pub community_cards: Vec<JsCard>,
    pub pot: f64,
    pub seats: Vec<JsSeat>,
    pub state_confidence: f32,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JsRecOutput {
    pub action: String,
    pub amount: f64,
    pub confidence: f64,
    pub distribution: std::collections::HashMap<String, f64>,
    pub ev: f64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StateUpdateEvent {
    pub table_id: String,
    pub state: JsTableState,
    pub timestamp_ms: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecommendationEvent {
    pub table_id: String,
    pub recommendation: JsRecOutput,
    pub timestamp_ms: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ErrorEvent {
    pub table_id: Option<String>,
    pub message: String,
}

// =============================================================================
// 字符串化辅助（JS 那边期待 string，不是 enum）
// =============================================================================

pub fn street_to_str(s: Street) -> &'static str {
    match s {
        Street::Preflop => "preflop",
        Street::Flop => "flop",
        Street::Turn => "turn",
        Street::River => "river",
        Street::Showdown => "showdown",
    }
}

pub fn phase_to_str(p: TablePhase) -> &'static str {
    match p {
        TablePhase::Waiting => "waiting",
        TablePhase::Playing => "playing",
        TablePhase::Showdown => "showdown",
        TablePhase::Cleanup => "cleanup",
    }
}

pub fn status_to_str(s: SeatStatus) -> &'static str {
    match s {
        SeatStatus::Empty => "empty",
        SeatStatus::SittingOut => "sittingOut",
        SeatStatus::Active => "active",
        SeatStatus::Folded => "folded",
        SeatStatus::AllIn => "allIn",
    }
}

pub fn action_to_str(a: &ActionType) -> String {
    match a {
        ActionType::Fold => "fold".into(),
        ActionType::Check => "check".into(),
        ActionType::Call => "call".into(),
        ActionType::Bet(amt) => format!("bet:{:.2}", amt),
        ActionType::Raise(amt) => format!("raise:{:.2}", amt),
        ActionType::AllIn(amt) => format!("allIn:{:.2}", amt),
        ActionType::PostBlind(BlindKind::SmallBlind) => "postSb".into(),
        ActionType::PostBlind(BlindKind::BigBlind) => "postBb".into(),
        ActionType::PostBlind(BlindKind::Straddle) => "postStraddle".into(),
        ActionType::PostBlind(BlindKind::Ante) => "postAnte".into(),
    }
}

pub fn js_seat_id(s: SeatId) -> u8 {
    s.0
}

#[cfg(test)]
mod tests {
    use super::*;
    use tf_core::{Rank, Suit};

    #[test]
    fn test_js_card_from_card() {
        let card = Card { suit: Suit::Spades, rank: Rank::Ace, confidence: 1.0 };
        let js: JsCard = card.into();
        assert_eq!(js.suit, "s");
        assert_eq!(js.rank, "A");
    }

    #[test]
    fn test_street_to_str() {
        assert_eq!(street_to_str(Street::Preflop), "preflop");
        assert_eq!(street_to_str(Street::Flop), "flop");
        assert_eq!(street_to_str(Street::Turn), "turn");
        assert_eq!(street_to_str(Street::River), "river");
        assert_eq!(street_to_str(Street::Showdown), "showdown");
    }

    #[test]
    fn test_phase_to_str() {
        assert_eq!(phase_to_str(TablePhase::Waiting), "waiting");
        assert_eq!(phase_to_str(TablePhase::Playing), "playing");
    }

    #[test]
    fn test_status_to_str() {
        assert_eq!(status_to_str(SeatStatus::Active), "active");
        assert_eq!(status_to_str(SeatStatus::Folded), "folded");
        assert_eq!(status_to_str(SeatStatus::AllIn), "allIn");
    }

    #[test]
    fn test_action_to_str() {
        assert_eq!(action_to_str(&ActionType::Fold), "fold");
        assert_eq!(action_to_str(&ActionType::Call), "call");
        assert!(action_to_str(&ActionType::Bet(50.0)).starts_with("bet:"));
    }

    #[test]
    fn test_js_table_state_serialize() {
        let state = JsTableState {
            table_id: "t1".into(),
            phase: "playing".into(),
            street: "preflop".into(),
            hand_number: 1,
            dealer_seat: Some(0),
            hero_seat: Some(2),
            hole_cards: Some(vec![JsCard { suit: "s".into(), rank: "A".into() }]),
            community_cards: vec![],
            pot: 100.0,
            seats: vec![],
            state_confidence: 0.95,
        };
        let json = serde_json::to_string(&state).unwrap();
        assert!(json.contains("\"tableId\":\"t1\""));
        assert!(json.contains("\"handNumber\":1"));
    }

    #[test]
    fn test_js_seat_id() {
        assert_eq!(js_seat_id(SeatId::new(5)), 5);
    }
}
