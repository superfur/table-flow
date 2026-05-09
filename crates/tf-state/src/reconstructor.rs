//! `ActionReconstructor` —— 从 stack/pot 变化推导玩家动作。
//!
//! 这是"State Derivation"的核心模块：**不通过 OCR 识别按钮文字**，
//! 而是基于 stack diff + pot diff + seat status change 三路信号交叉验证。

use serde::{Deserialize, Serialize};

use tf_core::ReconstructedAction;

use crate::state::TableState;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReconConfig {
    pub min_stack_change: f64,
    pub min_pot_change: f64,
    pub debounce_ms: u64,
    pub confidence_threshold: f32,
}

impl Default for ReconConfig {
    fn default() -> Self {
        Self {
            min_stack_change: 0.5,
            min_pot_change: 0.5,
            debounce_ms: 80,
            confidence_threshold: 0.6,
        }
    }
}

/// Reconstructor 接受当前帧的"特征变化集"作为输入。
/// 注意：这里我们用 tf-vision 的类型，但通过结构性传参避免在 tf-state 中
/// 反向依赖 tf-vision —— 调用者构造 `ReconInput` 然后传进来。
#[derive(Debug, Clone)]
pub struct ReconInput {
    pub stack_changes: Vec<StackDelta>,
    pub pot_change: Option<PotDelta>,
    pub folded_seats: Vec<tf_core::SeatId>,
    pub allin_seats: Vec<tf_core::SeatId>,
}

#[derive(Debug, Clone)]
pub struct StackDelta {
    pub seat_id: tf_core::SeatId,
    pub delta: f64,
    pub confidence: f32,
}

#[derive(Debug, Clone)]
pub struct PotDelta {
    pub delta: f64,
    pub confidence: f32,
}

pub struct ActionReconstructor {
    pub config: ReconConfig,
}

impl ActionReconstructor {
    pub fn new(config: ReconConfig) -> Self {
        Self { config }
    }

    /// 主入口。
    /// TODO(detail-impl):
    ///   1. derive_from_stack_changes()
    ///      - All-in 必须最先判定（stack 归零优先）
    ///      - PostBlind / Ante 单独识别（preflop & action_history 为空 & 金额匹配）
    ///      - Call/Bet/Raise 用 EPS 比较，不要直接 ==
    ///      - 不要伪造 Call 兜底
    ///   2. cross_validate_with_pot()
    ///      - 总动作金额 ≈ pot delta → bump confidence
    ///   3. derive_from_seat_status()
    ///      - Folded → ActionType::Fold
    ///      - AllIn → ActionType::AllIn(stack)
    ///   4. deduplicate(): 按 (seat_id, street, discriminant(action)) 去重
    pub fn reconstruct(
        &self,
        _prev_state: &TableState,
        _input: &ReconInput,
    ) -> Vec<ReconstructedAction> {
        todo!("ActionReconstructor::reconstruct")
    }
}
