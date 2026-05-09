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

use once_cell::sync::OnceCell;
use parking_lot::Mutex;

use tf_core::TfError;
use tf_table::TableManager;

use crate::types::{ErrorEvent, RecommendationEvent, StateUpdateEvent};

/// 占位 ThreadsafeFunction（detail-impl 阶段替换为 napi 真类型）
pub struct OpaqueTsfn<T> {
    _phantom: PhantomData<fn(T)>,
}

unsafe impl<T> Send for OpaqueTsfn<T> {}
unsafe impl<T> Sync for OpaqueTsfn<T> {}

impl<T> OpaqueTsfn<T> {
    pub fn placeholder() -> Self {
        Self { _phantom: PhantomData }
    }
    /// TODO(detail-impl): tsfn.call(Ok(value), NonBlocking)
    pub fn call(&self, _value: T) -> Result<(), TfError> {
        todo!("OpaqueTsfn::call")
    }
}

pub struct NapiBridge {
    pub manager: Arc<TableManager>,
    state_callback: Mutex<Option<OpaqueTsfn<StateUpdateEvent>>>,
    rec_callback: Mutex<Option<OpaqueTsfn<RecommendationEvent>>>,
    error_callback: Mutex<Option<OpaqueTsfn<ErrorEvent>>>,
}

static BRIDGE: OnceCell<Arc<NapiBridge>> = OnceCell::new();

impl NapiBridge {
    /// 单例初始化
    /// TODO(detail-impl): TableManager::new + 启动事件转发任务
    pub fn init(_manager: Arc<TableManager>) -> Result<Arc<Self>, TfError> {
        todo!("NapiBridge::init")
    }

    pub fn get() -> Result<Arc<Self>, TfError> {
        BRIDGE
            .get()
            .cloned()
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
}

/// 暴露给 commands.rs 的便利函数
pub fn get_bridge() -> Result<Arc<NapiBridge>, TfError> {
    NapiBridge::get()
}
