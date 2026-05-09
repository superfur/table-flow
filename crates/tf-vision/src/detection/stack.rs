//! 筹码追踪（OCR 数字主路径 + 像素面积 fallback）

use std::collections::{HashMap, VecDeque};
use std::sync::Arc;

use tf_core::{Frame, SeatId, TfError};
use tf_inference::Ocr;

use crate::features::StackChange;
use crate::pipeline::SeatRoi;

#[derive(Debug, Clone)]
pub struct StackBaseline {
    pub seat_id: SeatId,
    pub known_value: Option<f64>,
    pub pixel_count: f64,
    pub calibration_factor: f64,
}

#[derive(Debug, Clone)]
pub struct StackSnapshot {
    pub seat_id: SeatId,
    pub pixel_area: f64,
    pub estimated_value: f64,
    pub confidence: f32,
    pub timestamp_ms: i64,
}

pub struct StackTracker {
    pub baselines: HashMap<SeatId, StackBaseline>,
    pub history: HashMap<SeatId, VecDeque<StackSnapshot>>,
    pub history_window: usize,
}

impl Default for StackTracker {
    fn default() -> Self {
        Self {
            baselines: HashMap::new(),
            history: HashMap::new(),
            history_window: 30,
        }
    }
}

impl StackTracker {
    /// 主路径：digit OCR；fallback：像素面积。
    /// TODO(detail-impl)
    pub async fn track(
        &mut self,
        _seat_rois: &[SeatRoi],
        _frame: &Frame,
        _ocr: Option<&Arc<dyn Ocr>>,
    ) -> Result<Vec<StackChange>, TfError> {
        todo!("StackTracker::track")
    }

    /// 用一个已知的 stack 数值校准 pixel↔value 映射
    /// TODO(detail-impl)
    pub fn calibrate_from_known_value(
        &mut self,
        _seat_id: SeatId,
        _known_stack: f64,
        _frame: &Frame,
        _stack_roi: tf_core::Rect,
    ) -> Result<(), TfError> {
        todo!("StackTracker::calibrate_from_known_value")
    }
}
