//! `TableHandle` —— 单桌生命周期。

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use parking_lot::Mutex;
use tokio::task::JoinHandle;

use tf_core::{TableCalibration, TableId, TfError};
use tf_inference::InferencePool;
use tf_rec::RecEngine;
use tf_state::TableStateMachine;

#[derive(Clone, Default)]
pub struct CancelToken(Arc<AtomicBool>);

impl CancelToken {
    pub fn cancel(&self) {
        self.0.store(true, Ordering::SeqCst);
    }
    pub fn is_cancelled(&self) -> bool {
        self.0.load(Ordering::SeqCst)
    }
}

pub struct TableHandle {
    pub table_id: TableId,
    state_machine: Arc<Mutex<TableStateMachine>>,
    cancel: CancelToken,
    _tasks: Vec<JoinHandle<()>>,
}

impl TableHandle {
    pub async fn start(
        table_id: TableId,
        _calibration: TableCalibration,
        _inference_pool: Arc<InferencePool>,
        _rec_engine: Arc<dyn RecEngine>,
    ) -> Result<Self, TfError> {
        let state_machine = Arc::new(Mutex::new(TableStateMachine::new(table_id.clone())));

        Ok(Self {
            table_id,
            state_machine,
            cancel: CancelToken::default(),
            _tasks: Vec::new(),
        })
    }

    pub async fn shutdown(self) -> Result<(), TfError> {
        self.cancel.cancel();

        for handle in self._tasks {
            handle.abort();
        }

        Ok(())
    }

    pub async fn recover(&self) -> Result<(), TfError> {
        let mut sm = self.state_machine.lock();
        sm.reset_to_waiting();
        Ok(())
    }

    pub fn state_machine(&self) -> Arc<Mutex<TableStateMachine>> {
        self.state_machine.clone()
    }

    pub fn cancel_token(&self) -> CancelToken {
        self.cancel.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tf_core::{
        BlindsInfo, DigitOcrRegions, NormalizedRect, SeatCalibration, SeatId,
    };
    use tf_rec::MockRecEngine;

    fn make_calibration() -> TableCalibration {
        TableCalibration {
            resolution: (1920, 1080),
            hole_card_positions: [
                NormalizedRect::new(0.1, 0.5, 0.05, 0.08),
                NormalizedRect::new(0.17, 0.5, 0.05, 0.08),
            ],
            community_card_positions: [
                NormalizedRect::new(0.35, 0.4, 0.05, 0.08),
                NormalizedRect::new(0.42, 0.4, 0.05, 0.08),
                NormalizedRect::new(0.49, 0.4, 0.05, 0.08),
                NormalizedRect::new(0.56, 0.4, 0.05, 0.08),
                NormalizedRect::new(0.63, 0.4, 0.05, 0.08),
            ],
            pot_position: NormalizedRect::new(0.45, 0.3, 0.1, 0.04),
            seat_positions: vec![SeatCalibration {
                seat_id: SeatId::new(0),
                seat_region: NormalizedRect::new(0.0, 0.0, 0.1, 0.1),
                stack_region: NormalizedRect::new(0.0, 0.08, 0.1, 0.03),
                bet_region: NormalizedRect::new(0.05, 0.12, 0.05, 0.03),
                avatar_region: NormalizedRect::new(0.0, 0.0, 0.05, 0.05),
                card_region: None,
            }],
            dealer_button_region: NormalizedRect::new(0.3, 0.35, 0.03, 0.03),
            action_button_regions: [
                NormalizedRect::new(0.4, 0.85, 0.08, 0.04),
                NormalizedRect::new(0.5, 0.85, 0.08, 0.04),
                NormalizedRect::new(0.6, 0.85, 0.08, 0.04),
                NormalizedRect::new(0.7, 0.85, 0.08, 0.04),
            ],
            hero_seat: Some(SeatId::new(0)),
            blinds: BlindsInfo::default(),
            digit_ocr_regions: DigitOcrRegions::default(),
            theme_id: "test".to_string(),
        }
    }

    fn make_inference_pool() -> Arc<InferencePool> {
        Arc::new(
            InferencePool::new(tf_core::InferenceConfig {
                card_model_path: "/tmp/card.onnx".into(),
                digit_model_path: "/tmp/digit.onnx".into(),
                onnx_intra_threads: 1,
                onnx_inter_threads: 1,
                card_session_count: 1,
                digit_session_count: 1,
                use_gpu: false,
            })
            .unwrap(),
        )
    }

    fn make_rec_engine() -> Arc<dyn RecEngine> {
        Arc::new(MockRecEngine::new(tf_rec::RecOutput {
            action: "fold".to_string(),
            amount: 0.0,
            confidence: 0.5,
            distribution: Default::default(),
            ev: 0.0,
            processing_time_ms: 0.0,
        }))
    }

    #[tokio::test]
    async fn test_start_creates_handle() {
        let handle = TableHandle::start(
            "test-table".to_string(),
            make_calibration(),
            make_inference_pool(),
            make_rec_engine(),
        )
        .await
        .unwrap();

        assert_eq!(handle.table_id, "test-table");
        assert!(!handle.cancel_token().is_cancelled());
    }

    #[tokio::test]
    async fn test_shutdown_sets_cancel() {
        let handle = TableHandle::start(
            "test-table".to_string(),
            make_calibration(),
            make_inference_pool(),
            make_rec_engine(),
        )
        .await
        .unwrap();

        let token = handle.cancel_token();
        handle.shutdown().await.unwrap();
        assert!(token.is_cancelled());
    }

    #[tokio::test]
    async fn test_recover_resets_state() {
        let handle = TableHandle::start(
            "test-table".to_string(),
            make_calibration(),
            make_inference_pool(),
            make_rec_engine(),
        )
        .await
        .unwrap();

        handle.recover().await.unwrap();
        let sm_arc = handle.state_machine();
        let sm = sm_arc.lock();
        assert_eq!(sm.state().table_id, "test-table");
    }

    #[test]
    fn test_cancel_token() {
        let token = CancelToken::default();
        assert!(!token.is_cancelled());
        token.cancel();
        assert!(token.is_cancelled());
    }

    #[test]
    fn test_cancel_token_clone() {
        let token = CancelToken::default();
        let clone = token.clone();
        token.cancel();
        assert!(clone.is_cancelled());
    }
}
