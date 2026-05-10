//! `TableManager` —— 多桌编排核心。

use std::collections::HashMap;
use std::sync::Arc;

use parking_lot::RwLock;
use tokio::sync::broadcast;

use tf_core::{ManagerConfig, ManagerEvent, TableCalibration, TableId, TfError};
use tf_inference::InferencePool;
use tf_rec::{MockRecEngine, RecEngine};

use crate::handle::TableHandle;

pub struct TableManager {
    config: ManagerConfig,
    tables: Arc<RwLock<HashMap<TableId, TableHandle>>>,
    inference_pool: Arc<InferencePool>,
    rec_engine: Arc<dyn RecEngine>,
    event_tx: broadcast::Sender<ManagerEvent>,
}

impl TableManager {
    pub fn new(config: ManagerConfig) -> Result<Self, TfError> {
        let inference_pool = Arc::new(InferencePool::new(config.inference_config.clone())?);

        let rec_engine: Arc<dyn RecEngine> = Arc::new(MockRecEngine::new(tf_rec::RecOutput {
            action: "fold".to_string(),
            amount: 0.0,
            confidence: 0.5,
            distribution: Default::default(),
            ev: 0.0,
            processing_time_ms: 0.0,
        }));

        let (event_tx, _) = broadcast::channel(256);

        Ok(Self {
            config,
            tables: Arc::new(RwLock::new(HashMap::new())),
            inference_pool,
            rec_engine,
            event_tx,
        })
    }

    pub fn with_rec_engine(
        config: ManagerConfig,
        rec_engine: Arc<dyn RecEngine>,
    ) -> Result<Self, TfError> {
        let inference_pool = Arc::new(InferencePool::new(config.inference_config.clone())?);
        let (event_tx, _) = broadcast::channel(256);

        Ok(Self {
            config,
            tables: Arc::new(RwLock::new(HashMap::new())),
            inference_pool,
            rec_engine,
            event_tx,
        })
    }

    pub async fn start_table(
        &self,
        table_id: TableId,
        calibration: TableCalibration,
    ) -> Result<(), TfError> {
        {
            let tables = self.tables.read();
            if tables.contains_key(&table_id) {
                return Err(TfError::Config(format!(
                    "Table {} already started",
                    table_id
                )));
            }
        }

        let handle = TableHandle::start(
            table_id.clone(),
            calibration,
            self.inference_pool.clone(),
            self.rec_engine.clone(),
        )
        .await?;

        self.tables.write().insert(table_id, handle);

        Ok(())
    }

    pub async fn stop_table(&self, table_id: &TableId) -> Result<(), TfError> {
        let handle = self
            .tables
            .write()
            .remove(table_id)
            .ok_or_else(|| TfError::TableNotFound(table_id.clone()))?;

        handle.shutdown().await
    }

    pub fn list_tables(&self) -> Vec<TableId> {
        self.tables.read().keys().cloned().collect()
    }

    pub fn subscribe(&self) -> broadcast::Receiver<ManagerEvent> {
        self.event_tx.subscribe()
    }

    pub async fn shutdown_all(&self) -> Result<(), TfError> {
        let handles: Vec<(TableId, TableHandle)> = {
            let mut tables = self.tables.write();
            tables.drain().collect()
        };

        for (id, handle) in handles {
            if let Err(e) = handle.shutdown().await {
                tracing::warn!("Error shutting down table {}: {:?}", id, e);
            }
        }

        Ok(())
    }

    pub fn config(&self) -> &ManagerConfig {
        &self.config
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tf_core::{
        BlindsInfo, CaptureBackend, DigitOcrRegions, InferenceConfig, NormalizedRect,
        SeatCalibration, SeatId, ThreadConfig,
    };

    fn make_config() -> ManagerConfig {
        ManagerConfig {
            max_tables: 4,
            fps_per_table: 30,
            capture_backend: CaptureBackend::Dxgi,
            thread_config: ThreadConfig::default(),
            inference_config: InferenceConfig {
                card_model_path: "/tmp/card.onnx".into(),
                digit_model_path: "/tmp/digit.onnx".into(),
                onnx_intra_threads: 1,
                onnx_inter_threads: 1,
                card_session_count: 1,
                digit_session_count: 1,
                use_gpu: false,
            },
        }
    }

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

    #[test]
    fn test_manager_new() {
        let config = make_config();
        let mgr = TableManager::new(config).unwrap();
        assert!(mgr.list_tables().is_empty());
    }

    #[test]
    fn test_manager_subscribe() {
        let config = make_config();
        let mgr = TableManager::new(config).unwrap();
        let _rx = mgr.subscribe();
    }

    #[tokio::test]
    async fn test_start_and_list_table() {
        let config = make_config();
        let mgr = TableManager::new(config).unwrap();
        let cal = make_calibration();
        mgr.start_table("table-1".to_string(), cal)
            .await
            .unwrap();
        let tables = mgr.list_tables();
        assert_eq!(tables.len(), 1);
        assert!(tables.contains(&"table-1".to_string()));
    }

    #[tokio::test]
    async fn test_stop_table() {
        let config = make_config();
        let mgr = TableManager::new(config).unwrap();
        let cal = make_calibration();
        mgr.start_table("table-1".to_string(), cal)
            .await
            .unwrap();
        mgr.stop_table(&"table-1".to_string()).await.unwrap();
        assert!(mgr.list_tables().is_empty());
    }

    #[tokio::test]
    async fn test_stop_nonexistent_table() {
        let config = make_config();
        let mgr = TableManager::new(config).unwrap();
        let result = mgr.stop_table(&"nonexistent".to_string()).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_shutdown_all() {
        let config = make_config();
        let mgr = TableManager::new(config).unwrap();
        let cal = make_calibration();
        mgr.start_table("t1".to_string(), cal.clone())
            .await
            .unwrap();
        mgr.start_table("t2".to_string(), cal)
            .await
            .unwrap();
        mgr.shutdown_all().await.unwrap();
        assert!(mgr.list_tables().is_empty());
    }
}
