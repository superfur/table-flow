//! `TableHandle` —— 单桌生命周期。

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use parking_lot::Mutex;
use tokio::task::JoinHandle;

use tf_core::{TableCalibration, TableId, TfError};
use tf_inference::InferencePool;
use tf_rec::RecEngine;
use tf_state::TableStateMachine;

/// 简易取消信号（占位）。
/// detail-impl 阶段可换成 `tokio_util::sync::CancellationToken`。
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
    /// vision pipeline + state pump + rec pump 的 JoinHandle 集合
    tasks: Vec<JoinHandle<()>>,
}

impl TableHandle {
    /// 启动一桌完整 pipeline（capture → vision → state → rec）。
    /// TODO(detail-impl):
    ///   1. 创建 channel：frame_tx/rx, feature_tx/rx
    ///   2. 启动 VisionPipeline::run() 任务
    ///   3. 启动 state pump：feature_rx → ActionReconstructor → StateMachine.process_event
    ///   4. 启动 rec pump：state 变化 → build_rec_input → engine.recommend → emit ManagerEvent
    ///   5. 持有 JoinHandle 以便 shutdown 时 join
    pub async fn start(
        _table_id: TableId,
        _calibration: TableCalibration,
        _inference_pool: Arc<InferencePool>,
        _rec_engine: Arc<dyn RecEngine>,
    ) -> Result<Self, TfError> {
        todo!("TableHandle::start")
    }

    /// 优雅关闭。
    /// TODO(detail-impl): cancel.cancel() → join_all(tasks) with timeout
    pub async fn shutdown(self) -> Result<(), TfError> {
        todo!("TableHandle::shutdown")
    }

    /// recovery：连续 N 次错误后被调用
    /// TODO(detail-impl): rediscover_window + trigger_recalibration + reset_to_waiting
    pub async fn recover(&self) -> Result<(), TfError> {
        todo!("TableHandle::recover")
    }

    pub fn state_machine(&self) -> Arc<Mutex<TableStateMachine>> {
        self.state_machine.clone()
    }

    pub fn cancel_token(&self) -> CancelToken {
        self.cancel.clone()
    }
}
