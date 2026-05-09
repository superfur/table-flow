//! Pot 追踪（OCR 主路径 + 上一帧 hash 跳过）

use std::collections::VecDeque;
use std::sync::Arc;

use tf_core::{Frame, Rect, TfError};
use tf_inference::Ocr;

use crate::features::PotChange;

pub struct PotTracker {
    pub last_value: f64,
    pub last_pixel_hash: u64,
    pub change_history: VecDeque<PotChange>,
}

impl Default for PotTracker {
    fn default() -> Self {
        Self {
            last_value: 0.0,
            last_pixel_hash: 0,
            change_history: VecDeque::with_capacity(64),
        }
    }
}

impl PotTracker {
    /// 计算 pot 区域的 pixel hash（CRC32），相同则跳过 OCR。
    /// TODO(detail-impl)
    pub fn compute_hash(&self, _pot_img: &Frame) -> u64 {
        todo!("PotTracker::compute_hash")
    }

    /// 主路径：digit OCR；hash 不变直接返回 None。
    /// TODO(detail-impl)
    pub async fn track(
        &mut self,
        _pot_roi: &Rect,
        _frame: &Frame,
        _ocr: Option<&Arc<dyn Ocr>>,
    ) -> Result<Option<PotChange>, TfError> {
        todo!("PotTracker::track")
    }
}
