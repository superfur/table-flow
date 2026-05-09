//! `TableManager` —— 多桌编排核心。

use std::collections::HashMap;
use std::sync::Arc;

use parking_lot::RwLock;
use tokio::sync::broadcast;

use tf_core::{ManagerConfig, ManagerEvent, TableCalibration, TableId, TfError};
use tf_inference::InferencePool;
use tf_rec::RecEngine;

use crate::handle::TableHandle;

pub struct TableManager {
    config: ManagerConfig,
    tables: Arc<RwLock<HashMap<TableId, TableHandle>>>,
    inference_pool: Arc<InferencePool>,
    rec_engine: Arc<dyn RecEngine>,
    event_tx: broadcast::Sender<ManagerEvent>,
}

impl TableManager {
    /// TODO(detail-impl):
    ///   - 初始化 InferencePool
    ///   - 初始化 RecSidecar 并 wrap 成 Arc<dyn RecEngine>
    ///   - 创建 broadcast channel
    pub fn new(_config: ManagerConfig) -> Result<Self, TfError> {
        todo!("TableManager::new")
    }

    /// 启动一桌 capture + state pipeline
    pub async fn start_table(
        &self,
        _table_id: TableId,
        _calibration: TableCalibration,
    ) -> Result<(), TfError> {
        todo!("TableManager::start_table")
    }

    pub async fn stop_table(&self, _table_id: &TableId) -> Result<(), TfError> {
        todo!("TableManager::stop_table")
    }

    pub fn list_tables(&self) -> Vec<TableId> {
        self.tables.read().keys().cloned().collect()
    }

    /// 订阅 manager 级事件
    pub fn subscribe(&self) -> broadcast::Receiver<ManagerEvent> {
        self.event_tx.subscribe()
    }

    pub async fn shutdown_all(&self) -> Result<(), TfError> {
        todo!("TableManager::shutdown_all")
    }

    pub fn config(&self) -> &ManagerConfig {
        &self.config
    }
}
