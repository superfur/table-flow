//! 轮廓分析（用于 auto-calibration、卡片定位）

use tf_core::{Frame, Rect, TfError};

#[derive(Debug, Clone)]
pub struct Contour {
    pub bounding_box: Rect,
    pub aspect_ratio: f32,
    pub area: u32,
}

pub struct ContourAnalyzer;

impl ContourAnalyzer {
    /// 在 frame 上找出所有轮廓
    /// TODO(detail-impl): cvtColor → threshold → findContours → boundingRect
    pub fn find_all(&self, _frame: &Frame) -> Result<Vec<Contour>, TfError> {
        todo!("ContourAnalyzer::find_all")
    }
}
