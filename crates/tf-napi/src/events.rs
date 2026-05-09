//! 暴露给 JS 的事件订阅入口。
//!
//! detail-impl 阶段：每个函数对应一个 `#[napi] pub fn on_xxx(callback: JsFunction)`
//! 内部调用 `callback.create_threadsafe_function(0, |ctx| Ok(vec![ctx.value]))`
//! 然后 `bridge.register_xxx_callback(tsfn)`。

use tf_core::TfError;

use crate::bridge::{get_bridge, OpaqueTsfn};
use crate::types::{ErrorEvent, RecommendationEvent, StateUpdateEvent};

pub fn on_state_update(callback: OpaqueTsfn<StateUpdateEvent>) -> Result<(), TfError> {
    let bridge = get_bridge()?;
    bridge.register_state_callback(callback);
    Ok(())
}

pub fn on_recommendation(callback: OpaqueTsfn<RecommendationEvent>) -> Result<(), TfError> {
    let bridge = get_bridge()?;
    bridge.register_rec_callback(callback);
    Ok(())
}

pub fn on_error(callback: OpaqueTsfn<ErrorEvent>) -> Result<(), TfError> {
    let bridge = get_bridge()?;
    bridge.register_error_callback(callback);
    Ok(())
}
