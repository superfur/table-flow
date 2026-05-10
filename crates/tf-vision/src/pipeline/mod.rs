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
use tf_inference::Ocr;

use crate::calibration::AutoCalibrator;
use crate::capture::FrameCapture;
use crate::detection::{
    CardDetector, DealerTracker, HeroDetector, PotTracker, SeatTracker, StackTracker,
};
use crate::features::ExtractedFeatures;

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
    pub ocr: Arc<dyn Ocr>,
    pub output_tx: Sender<ExtractedFeatures>,
}

#[async_trait]
pub trait VisionPipelineRun {
    async fn run(self) -> Result<(), TfError>;
}

#[async_trait]
impl VisionPipelineRun for VisionPipeline {
    async fn run(mut self) -> Result<(), TfError> {
        let mut prev_frame: Option<tf_core::Frame> = None;
        let mut frame_number: u64 = 0;

        loop {
            let captured = match self.capture.capture_frame().await {
                Ok(f) => f,
                Err(e) => {
                    tracing::warn!("Capture error: {:?}", e);
                    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                    continue;
                }
            };
            frame_number = captured.frame_number;

            let processed = self.preprocessor.process(&captured.frame)?;

            let changed = match &prev_frame {
                Some(prev) => self.diff_detector.has_significant_change(prev, &processed),
                None => true,
            };

            if !changed {
                prev_frame = Some(processed);
                continue;
            }

            let rois = self.roi_manager.extract(&processed)?;

            let cards = self
                .card_detector
                .detect(&rois.hole_cards, &rois.community_cards, &processed)
                .await
                .unwrap_or_default();

            let stacks = self
                .stack_tracker
                .track(&rois.player_seats, &processed, Some(&self.ocr))
                .await
                .unwrap_or_default();

            let pot = self
                .pot_tracker
                .track(&rois.pot_area, &processed, Some(&self.ocr))
                .await
                .unwrap_or(None);

            let seats = self
                .seat_tracker
                .track(&rois.player_seats, &processed)
                .unwrap_or_default();

            let dealer = self
                .dealer_tracker
                .detect(&rois.dealer_button, &rois.player_seats, &processed)
                .unwrap_or(None);

            let hero = self
                .hero_detector
                .detect(&rois.player_seats, &processed, self.card_detector.as_ref())
                .await
                .unwrap_or(None);

            let raw = crate::features::RawFeatures {
                cards,
                stacks,
                pot,
                seats,
                dealer,
                hero,
                timestamp_ms: captured.timestamp_ms,
            };

            let features = self.aggregator.merge(raw);

            if self.output_tx.send(features).is_err() {
                tracing::info!("Output channel closed, stopping pipeline");
                break;
            }

            prev_frame = Some(processed);
        }

        Ok(())
    }
}
