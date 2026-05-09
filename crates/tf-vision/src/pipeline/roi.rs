//! ROI 管理：把 `TableCalibration`（归一化坐标）映射成本帧的具体像素区域。

use tf_core::{Frame, Rect, SeatId, TableCalibration, TfError};

/// 一帧上所有需要分析的子区域
#[derive(Debug, Clone)]
pub struct TableRoi {
    pub hole_cards: [Rect; 2],
    pub community_cards: [Rect; 5],
    pub pot_area: Rect,
    pub player_seats: Vec<SeatRoi>,
    pub dealer_button: Rect,
    pub action_buttons: [Rect; 4],
}

#[derive(Debug, Clone)]
pub struct SeatRoi {
    pub seat_id: SeatId,
    pub seat_area: Rect,
    pub stack_area: Rect,
    pub bet_area: Rect,
    pub avatar_area: Rect,
    pub card_area: Option<Rect>,
}

pub struct RoiManager {
    pub calibration: TableCalibration,
    /// detail-impl 可在这里缓存上一次计算结果，避免重复 to_pixel_rect
    cached_resolution: Option<(u32, u32)>,
    cached_roi: Option<TableRoi>,
}

impl RoiManager {
    pub fn new(calibration: TableCalibration) -> Self {
        Self {
            calibration,
            cached_resolution: None,
            cached_roi: None,
        }
    }

    pub fn update_calibration(&mut self, calibration: TableCalibration) {
        self.calibration = calibration;
        self.cached_resolution = None;
        self.cached_roi = None;
    }

    /// TODO(detail-impl): 把 calibration 中的归一化坐标乘以 frame 分辨率，得到像素 ROI
    pub fn extract(&mut self, _frame: &Frame) -> Result<TableRoi, TfError> {
        todo!("RoiManager::extract")
    }
}
