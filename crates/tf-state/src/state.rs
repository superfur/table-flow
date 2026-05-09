//! `TableState` 与子结构 —— 单一状态真相来源。

use serde::{Deserialize, Serialize};

use tf_core::{
    ActionType, BlindsInfo, Card, SeatId, SeatStatus, Street, TableId, TablePhase,
};

/// 完整的牌桌快照。
///
/// 由 `TableStateMachine` 维护并对外只读。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableState {
    pub table_id: TableId,
    pub phase: TablePhase,
    pub street: Street,
    pub hand_number: u64,
    pub dealer_seat: Option<SeatId>,
    pub hero_seat: Option<SeatId>,
    pub hole_cards: Option<[Card; 2]>,
    pub community_cards: Vec<Card>,
    pub pot: PotInfo,
    pub seats: Vec<SeatState>,
    pub action_history: Vec<ActionRecord>,
    pub current_player_turn: Option<SeatId>,
    pub blinds: BlindsInfo,
    /// epoch ms
    pub last_update_ms: i64,
    pub state_confidence: f32,
}

impl TableState {
    pub fn initial(table_id: TableId) -> Self {
        Self {
            table_id,
            phase: TablePhase::Waiting,
            street: Street::Preflop,
            hand_number: 0,
            dealer_seat: None,
            hero_seat: None,
            hole_cards: None,
            community_cards: Vec::new(),
            pot: PotInfo::default(),
            seats: Vec::new(),
            action_history: Vec::new(),
            current_player_turn: None,
            blinds: BlindsInfo::default(),
            last_update_ms: 0,
            state_confidence: 1.0,
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PotInfo {
    pub main_pot: f64,
    pub side_pots: Vec<SidePot>,
    pub total: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SidePot {
    pub amount: f64,
    pub eligible_seats: Vec<SeatId>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SeatState {
    pub seat_id: SeatId,
    pub status: SeatStatus,
    pub stack: f64,
    pub current_bet: f64,
    pub total_bet_this_hand: f64,
    pub last_action: Option<ActionRecord>,
    pub is_hero: bool,
    pub has_cards: bool,
}

impl SeatState {
    pub fn new(seat_id: SeatId, stack: f64) -> Self {
        Self {
            seat_id,
            status: SeatStatus::Empty,
            stack,
            current_bet: 0.0,
            total_bet_this_hand: 0.0,
            last_action: None,
            is_hero: false,
            has_cards: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionRecord {
    pub seat_id: SeatId,
    pub action: ActionType,
    pub amount: f64,
    pub street: Street,
    pub seq: u32,
    pub confidence: f32,
}
