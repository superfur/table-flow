//! 跨 crate 的事件类型。
//!
//! 三层事件：
//!   - `TableEvent`：vision → state machine（来自视觉特征推导）
//!   - `StateTransition`：state machine 内部的状态转移记录
//!   - `ManagerEvent`：table manager → 上游（IPC bridge）的高层事件
//!
//! 具体业务方法（`process_event` 等）在 `tf-state` 中。
//! 这里只放纯数据。

use serde::{Deserialize, Serialize};

use crate::types::{ActionSource, ActionType, BlindKind, Card, SeatId, SeatStatus, Street};

// =============================================================================
// 视觉层 → 状态层
// =============================================================================

/// Vision 推导出的事件，喂给 `TableStateMachine::process_event`
#[derive(Debug, Clone)]
pub enum TableEvent {
    NewHandDetected {
        dealer_seat: SeatId,
    },
    HoleCardsDetected {
        cards: [Card; 2],
    },
    CommunityCardsChanged {
        cards: Vec<Card>,
        street: Street,
    },
    ActionReconstructed {
        action: ReconstructedAction,
    },
    PotChanged {
        new_total: f64,
        delta: f64,
    },
    SeatStatusChanged {
        seat_id: SeatId,
        new_status: SeatStatus,
    },
    DealerButtonMoved {
        new_seat: SeatId,
    },
    Timeout,
}

/// 由 `ActionReconstructor` 输出的"动作候选"。
/// 进入 state machine 后才被记录为 `ActionRecord`。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReconstructedAction {
    pub seat_id: SeatId,
    pub action_type: ActionType,
    /// 视觉推导出的金额（可能为 None，例如 Fold/Check）
    pub amount: Option<f64>,
    pub street: Street,
    /// epoch ms（不用 `Instant`，方便跨进程序列化）
    pub timestamp_ms: i64,
    pub confidence: f32,
    pub source: ActionSource,
}

/// 盲注事件（独立于 ActionReconstructed 以便上游显式区分自愿动作）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostedBlind {
    pub seat_id: SeatId,
    pub kind: BlindKind,
    pub amount: f64,
}

// =============================================================================
// 状态层内部
// =============================================================================

#[derive(Debug, Clone)]
pub enum StateTransition {
    HandStarted { hand_number: u64 },
    StreetChanged { from: Street, to: Street },
    ActionRecorded(ActionRecordSummary),
    HandCompleted { result: HandResult },
    StateConfidenceUpdated { new_confidence: f32 },
}

/// `StateTransition::ActionRecorded` 的精简载荷
/// （完整 `ActionRecord` 在 `tf-state` 中定义并持久化）
#[derive(Debug, Clone)]
pub struct ActionRecordSummary {
    pub seat_id: SeatId,
    pub action: ActionType,
    pub amount: f64,
    pub street: Street,
    pub seq: u32,
    pub confidence: f32,
}

#[derive(Debug, Clone)]
pub struct HandResult {
    pub hand_number: u64,
    /// 各座位的净盈亏（正赢负输）
    pub seat_pnl: Vec<(SeatId, f64)>,
}

// =============================================================================
// 表管理层 → 上游
// =============================================================================

#[derive(Debug, Clone)]
pub enum ManagerEvent {
    TableDiscovered {
        table_id: String,
        window_title: String,
    },
    TableLost {
        table_id: String,
    },
    /// 用 Box 避免单变体过大
    StateUpdated {
        table_id: String,
        snapshot_json: String,
    },
    RecommendationReady {
        table_id: String,
        output_json: String,
    },
    Error {
        table_id: Option<String>,
        message: String,
    },
}
