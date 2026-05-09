//! 庄家按钮检测

use tf_core::{Frame, Rect, SeatId, TfError};

pub struct DealerTracker {
    /// detail-impl 阶段填入：模板图像 + 各座位的"庄家位"参考点
    pub last_position: Option<SeatId>,
}

impl Default for DealerTracker {
    fn default() -> Self {
        Self { last_position: None }
    }
}

impl DealerTracker {
    /// TODO(detail-impl):
    ///   - 在 dealer_roi 内做模板匹配（TM_CCOEFF_NORMED）
    ///   - max_val > 0.8 时返回最近的 SeatId
    pub fn detect(
        &mut self,
        _dealer_roi: &Rect,
        _frame: &Frame,
    ) -> Result<Option<SeatId>, TfError> {
        todo!("DealerTracker::detect")
    }
}
