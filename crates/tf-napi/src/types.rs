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
