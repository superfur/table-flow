//! Frame processing pipeline 主模块。

pub mod aggregator;
pub mod diff;
pub mod preprocessor;
pub mod roi;

pub use aggregator::*;
pub use diff::*;
pub use preprocessor::*;
pub use roi::*;

use std::sync::Arc;

use async_trait::async_trait;
use flume::Sender;

use tf_core::{TableId, TfError};
use tf_inference::{Ocr, OcrAssistant};

use crate::calibration::AutoCalibrator;
use crate::capture::FrameCapture;
use crate::detection::{
    CardDetector, DealerTracker, HeroDetector, PotTracker, SeatTracker, StackTracker,
};
use crate::features::ExtractedFeatures;

/// 视觉 pipeline 的主结构 —— 一桌一个实例。
///
/// 由 `tf-table::TableHandle` 持有并驱动。
pub struct VisionPipeline {
    pub table_id: TableId,
    pub capture: Box<dyn FrameCapture>,
    pub preprocessor: Preprocessor,
    pub roi_manager: RoiManager,
    pub diff_detector: DiffDetector,
    pub card_detector: Box<dyn CardDetector>,
    pub stack_tracker: StackTracker,
    pub pot_tracker: PotTracker,
    pub seat_tracker: SeatTracker,
    pub dealer_tracker: DealerTracker,
    pub hero_detector: HeroDetector,
    pub auto_calibrator: Option<AutoCalibrator>,
    pub aggregator: FeatureAggregator,
    pub ocr: Arc<OcrAssistant>,
    pub output_tx: Sender<ExtractedFeatures>,
}

#[async_trait]
pub trait VisionPipelineRun {
    /// 跑主循环。在 cancel_token 触发或 output_tx 断开时返回。
    async fn run(self) -> Result<(), TfError>;
}

#[async_trait]
impl VisionPipelineRun for VisionPipeline {
    /// TODO(detail-impl):
    /// loop:
    ///   1. fps_limit.wait()
    ///   2. capture.capture_frame()
    ///   3. preprocessor.process()
    ///   4. diff_detector.has_significant_change()? 否则跳过
    ///   5. roi_manager.extract()
    ///   6. detection 各 tracker 并行 (rayon scope)
    ///   7. aggregator.merge() → ExtractedFeatures
    ///   8. output_tx.send()
    async fn run(self) -> Result<(), TfError> {
        let _ocr: Arc<dyn Ocr> = self.ocr.clone();
        todo!("VisionPipeline::run")
    }
}
