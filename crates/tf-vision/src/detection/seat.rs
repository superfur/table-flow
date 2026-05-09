//! 座位状态追踪（Empty / SittingOut / Active / Folded / AllIn）

use std::collections::HashMap;
use std::time::Instant;

use tf_core::{Frame, SeatId, SeatStatus, TfError};

use crate::features::SeatChange;
use crate::pipeline::SeatRoi;

#[derive(Debug, Clone)]
pub struct TrackedSeat {
    pub seat_id: SeatId,
    pub status: SeatStatus,
    pub last_seen_active: Instant,
}

pub struct SeatTracker {
    pub seat_states: HashMap<SeatId, TrackedSeat>,
}

impl Default for SeatTracker {
    fn default() -> Self {
        Self {
            seat_states: HashMap::new(),
        }
    }
}

impl SeatTracker {
    /// 比较各座位 ROI 与上一次状态，输出变化集
    /// TODO(detail-impl):
    ///   - 用亮度 / 颜色 / 标志元素（弃牌图标 / All-in 标记）综合判定
    pub fn track(
        &mut self,
        _seat_rois: &[SeatRoi],
        _frame: &Frame,
    ) -> Result<Vec<SeatChange>, TfError> {
        todo!("SeatTracker::track")
    }

    /// 单帧分类一个座位的状态
    pub fn classify_seat_status(&self, _seat_img: &Frame) -> Result<SeatStatus, TfError> {
        todo!("SeatTracker::classify_seat_status")
    }
}
