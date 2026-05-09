//! Hero 座位识别（手动校准 + 正面手牌检测 fallback）

use std::time::{Duration, Instant};

use tf_core::{Frame, SeatId, TfError};

use crate::detection::card::CardDetector;
use crate::pipeline::SeatRoi;

pub struct HeroDetector {
    /// 用户在校准阶段手动指定的 hero 座位（MVP 必填）
    pub manual_hero: Option<SeatId>,
    /// 上次自动检测出的 hero（用于在 confidence_window 内复用）
    pub last_detected: Option<(SeatId, Instant)>,
    pub confidence_window: Duration,
}

impl Default for HeroDetector {
    fn default() -> Self {
        Self {
            manual_hero: None,
            last_detected: None,
            confidence_window: Duration::from_secs(10),
        }
    }
}

impl HeroDetector {
    pub fn with_manual(seat: SeatId) -> Self {
        Self {
            manual_hero: Some(seat),
            ..Default::default()
        }
    }

    /// 优先级：手动 > 正面牌检测 > 缓存。
    /// TODO(detail-impl)
    pub async fn detect(
        &mut self,
        _seat_rois: &[SeatRoi],
        _frame: &Frame,
        _card_detector: &dyn CardDetector,
    ) -> Result<Option<SeatId>, TfError> {
        todo!("HeroDetector::detect")
    }
}
