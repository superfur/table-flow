//! 暴露给 JS 的命令（对应 napi `#[napi]` async fn）。
//!
//! 架构骨架阶段：用普通 async fn 占位，签名与 JS 期望对齐。
//! detail-impl 阶段：在每个函数加 `#[napi]`，把 `serde_json::Value` 替换为
//! 具体的 JS 类型（`napi::JsObject` / 自动派生 ToNapiValue 的 struct）。

use serde_json::Value as JsonValue;

use tf_core::TfError;

use crate::bridge::get_bridge;

/// 启动一桌 capture（参数预期：`{ tableId, windowTitle, calibration? }`）
pub async fn start_capture(_config: JsonValue) -> Result<(), TfError> {
    let _bridge = get_bridge()?;
    todo!("commands::start_capture — bridge.manager.start_table(...)")
}

pub async fn stop_capture(_table_id: String) -> Result<(), TfError> {
    let _bridge = get_bridge()?;
    todo!("commands::stop_capture")
}

pub async fn discover_tables() -> Result<Vec<String>, TfError> {
    todo!("commands::discover_tables — TableDiscovery::scan().await")
}

pub async fn get_table_state(_table_id: String) -> Result<JsonValue, TfError> {
    let _bridge = get_bridge()?;
    todo!("commands::get_table_state — bridge.manager.snapshot(table_id)")
}

pub async fn calibrate_table(_table_id: String) -> Result<JsonValue, TfError> {
    let _bridge = get_bridge()?;
    todo!("commands::calibrate_table")
}

pub async fn shutdown() -> Result<(), TfError> {
    let _bridge = get_bridge()?;
    todo!("commands::shutdown")
}
