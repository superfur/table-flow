//! NapiBridge —— Rust ↔ JS 之间的状态/回调持有者。
//!
//! 设计：
//!   - 单例（`once_cell::sync::OnceCell<Arc<NapiBridge>>`）
//!   - 内部持有 `Arc<TableManager>` 和三组 ThreadsafeFunction 句柄
//!   - 后台线程通过 `emit_*` 把事件转发给 JS
//!
//! 架构骨架阶段：先用占位类型 `OpaqueTsfn<T>` 替代 `napi::ThreadsafeFunction`，
//! detail-impl 阶段引入 napi 后替换。

use std::marker::PhantomData;
use std::sync::Arc;

use parking_lot::Mutex;

use tf_core::TfError;
use tf_table::TableManager;

use crate::types::{ErrorEvent, RecommendationEvent, StateUpdateEvent};

pub struct OpaqueTsfn<T> {
    _phantom: PhantomData<fn(T)>,
}

unsafe impl<T> Send for OpaqueTsfn<T> {}
unsafe impl<T> Sync for OpaqueTsfn<T> {}

impl<T> OpaqueTsfn<T> {
    pub fn placeholder() -> Self {
        Self { _phantom: PhantomData }
    }
    pub fn call(&self, _value: T) -> Result<(), TfError> {
        Ok(())
    }
}

pub struct NapiBridge {
    pub manager: Arc<TableManager>,
    state_callback: Mutex<Option<OpaqueTsfn<StateUpdateEvent>>>,
    rec_callback: Mutex<Option<OpaqueTsfn<RecommendationEvent>>>,
    error_callback: Mutex<Option<OpaqueTsfn<ErrorEvent>>>,
}

static BRIDGE: Mutex<Option<Arc<NapiBridge>>> = Mutex::new(None);

#[cfg(test)]
pub static TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

impl NapiBridge {
    pub fn init(manager: Arc<TableManager>) -> Result<Arc<Self>, TfError> {
        let bridge = Arc::new(Self {
            manager,
            state_callback: Mutex::new(None),
            rec_callback: Mutex::new(None),
            error_callback: Mutex::new(None),
        });

        let mut guard = BRIDGE.lock();
        if guard.is_some() {
            return Err(TfError::Ipc("bridge already initialized".into()));
        }
        *guard = Some(bridge.clone());

        Ok(bridge)
    }

    pub fn get() -> Result<Arc<Self>, TfError> {
        BRIDGE
            .lock()
            .clone()
            .ok_or_else(|| TfError::Ipc("bridge not initialized".into()))
    }

    pub fn register_state_callback(&self, cb: OpaqueTsfn<StateUpdateEvent>) {
        *self.state_callback.lock() = Some(cb);
    }
    pub fn register_rec_callback(&self, cb: OpaqueTsfn<RecommendationEvent>) {
        *self.rec_callback.lock() = Some(cb);
    }
    pub fn register_error_callback(&self, cb: OpaqueTsfn<ErrorEvent>) {
        *self.error_callback.lock() = Some(cb);
    }

    pub fn emit_state_update(&self, event: StateUpdateEvent) -> Result<(), TfError> {
        if let Some(cb) = self.state_callback.lock().as_ref() {
            cb.call(event)?;
        }
        Ok(())
    }
    pub fn emit_recommendation(&self, event: RecommendationEvent) -> Result<(), TfError> {
        if let Some(cb) = self.rec_callback.lock().as_ref() {
            cb.call(event)?;
        }
        Ok(())
    }
    pub fn emit_error(&self, event: ErrorEvent) -> Result<(), TfError> {
        if let Some(cb) = self.error_callback.lock().as_ref() {
            cb.call(event)?;
        }
        Ok(())
    }

    pub fn reset() {
        *BRIDGE.lock() = None;
    }
}

pub fn get_bridge() -> Result<Arc<NapiBridge>, TfError> {
    NapiBridge::get()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tf_core::{
        CaptureBackend, InferenceConfig, ManagerConfig, ThreadConfig,
    };

    fn make_manager() -> Arc<TableManager> {
        Arc::new(
            TableManager::new(ManagerConfig {
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
            })
            .unwrap(),
        )
    }

    #[test]
    fn test_init_and_get() {
        let _guard = TEST_LOCK.lock().unwrap();
        NapiBridge::reset();
        let mgr = make_manager();
        let bridge = NapiBridge::init(mgr).unwrap();
        let got = NapiBridge::get().unwrap();
        assert!(Arc::ptr_eq(&bridge, &got));
    }

    #[test]
    fn test_init_twice_fails() {
        let _guard = TEST_LOCK.lock().unwrap();
        NapiBridge::reset();
        let mgr = make_manager();
        NapiBridge::init(mgr.clone()).unwrap();
        let result = NapiBridge::init(mgr);
        assert!(result.is_err());
    }

    #[test]
    fn test_get_before_init_fails() {
        let _guard = TEST_LOCK.lock().unwrap();
        NapiBridge::reset();
        let result = NapiBridge::get();
        assert!(result.is_err());
    }

    #[test]
    fn test_register_callbacks() {
        let _guard = TEST_LOCK.lock().unwrap();
        NapiBridge::reset();
        let mgr = make_manager();
        let bridge = NapiBridge::init(mgr).unwrap();
        bridge.register_state_callback(OpaqueTsfn::placeholder());
        bridge.register_rec_callback(OpaqueTsfn::placeholder());
        bridge.register_error_callback(OpaqueTsfn::placeholder());
        assert!(bridge.state_callback.lock().is_some());
        assert!(bridge.rec_callback.lock().is_some());
        assert!(bridge.error_callback.lock().is_some());
    }

    #[test]
    fn test_emit_without_callback_ok() {
        let _guard = TEST_LOCK.lock().unwrap();
        NapiBridge::reset();
        let mgr = make_manager();
        let bridge = NapiBridge::init(mgr).unwrap();
        bridge
            .emit_state_update(StateUpdateEvent {
                table_id: "t1".into(),
                state: crate::types::JsTableState {
                    table_id: "t1".into(),
                    phase: "playing".into(),
                    street: "preflop".into(),
                    hand_number: 1,
                    dealer_seat: None,
                    hero_seat: None,
                    hole_cards: None,
                    community_cards: vec![],
                    pot: 0.0,
                    seats: vec![],
                    state_confidence: 1.0,
                },
                timestamp_ms: 0,
            })
            .unwrap();
    }
}
