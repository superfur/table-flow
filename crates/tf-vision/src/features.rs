//! 各检测器输出的"特征"类型（变化集），以及聚合后的 `ExtractedFeatures`。

use serde::{Deserialize, Serialize};

use tf_core::{Card, SeatId, SeatStatus, Street, TableId};

// =============================================================================
// 单帧"原始特征"（各 detector 各自输出）
// =============================================================================

#[derive(Debug, Clone)]
pub struct RawFeatures {
    pub cards: CardDetectionResult,
    pub stacks: Vec<StackChange>,
    pub pot: Option<PotChange>,
    pub seats: Vec<SeatChange>,
    pub dealer: Option<SeatId>,
    pub hero: Option<SeatId>,
    /// 帧时间戳（epoch ms）
    pub timestamp_ms: i64,
}

#[derive(Debug, Clone, Default)]
pub struct CardDetectionResult {
    pub hole_cards: Option<[Card; 2]>,
    pub community_cards: Vec<Card>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StackChange {
    pub seat_id: SeatId,
    pub prev_estimated: f64,
    pub curr_estimated: f64,
    pub delta: f64,
    pub confidence: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PotChange {
    pub prev_value: f64,
    pub new_value: f64,
    pub delta: f64,
    pub timestamp_ms: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SeatChange {
    pub seat_id: SeatId,
    pub prev_status: Option<SeatStatus>,
    pub new_status: SeatStatus,
}

// =============================================================================
// 聚合后的"已提取特征"，发往 state machine
// =============================================================================

#[derive(Debug, Clone)]
pub struct ExtractedFeatures {
    pub table_id: TableId,
    pub timestamp_ms: i64,
    pub hole_cards: Option<[Card; 2]>,
    pub community_cards: Vec<Card>,
    pub street: Street,
    pub stack_changes: Vec<StackChange>,
    pub pot_change: Option<PotChange>,
    pub seat_changes: Vec<SeatChange>,
    pub dealer_seat: Option<SeatId>,
    pub hero_seat: Option<SeatId>,
}
