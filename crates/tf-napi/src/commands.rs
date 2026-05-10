//! 暴露给 JS 的命令（对应 napi `#[napi]` async fn）。
//!
//! 架构骨架阶段：用普通 async fn 占位，签名与 JS 期望对齐。
//! detail-impl 阶段：在每个函数加 `#[napi]`，把 `serde_json::Value` 替换为
//! 具体的 JS 类型（`napi::JsObject` / 自动派生 ToNapiValue 的 struct）。

use serde_json::Value as JsonValue;

use tf_core::TfError;
use tf_table::TableDiscovery;

use crate::bridge::get_bridge;

pub async fn start_capture(config: JsonValue) -> Result<(), TfError> {
    let bridge = get_bridge()?;

    let table_id = config
        .get("tableId")
        .and_then(|v| v.as_str())
        .ok_or_else(|| TfError::Ipc("start_capture: missing tableId".into()))?
        .to_string();

    let calibration = parse_calibration(&config)?;

    bridge
        .manager
        .start_table(table_id, calibration)
        .await?;

    Ok(())
}

pub async fn stop_capture(table_id: String) -> Result<(), TfError> {
    let bridge = get_bridge()?;
    bridge.manager.stop_table(&table_id).await
}

pub async fn discover_tables() -> Result<Vec<String>, TfError> {
    let tables = TableDiscovery::scan().await?;
    Ok(tables.into_iter().map(|t| t.table_id).collect())
}

pub async fn get_table_state(table_id: String) -> Result<JsonValue, TfError> {
    let bridge = get_bridge()?;
    let tables = bridge.manager.list_tables();
    if !tables.contains(&table_id) {
        return Err(TfError::TableNotFound(table_id));
    }
    Ok(serde_json::json!({ "tableId": table_id, "status": "active" }))
}

pub async fn calibrate_table(_table_id: String) -> Result<JsonValue, TfError> {
    Ok(serde_json::json!({ "status": "calibration_not_available_in_mvp" }))
}

pub async fn shutdown() -> Result<(), TfError> {
    let bridge = get_bridge()?;
    bridge.manager.shutdown_all().await
}

fn parse_calibration(config: &JsonValue) -> Result<tf_core::TableCalibration, TfError> {
    if let Some(cal) = config.get("calibration") {
        serde_json::from_value(cal.clone()).map_err(|e| {
            TfError::Config(format!("Invalid calibration JSON: {}", e))
        })
    } else {
        Ok(tf_core::TableCalibration {
            resolution: (1920, 1080),
            hole_card_positions: [
                tf_core::NormalizedRect::new(0.1, 0.5, 0.05, 0.08),
                tf_core::NormalizedRect::new(0.17, 0.5, 0.05, 0.08),
            ],
            community_card_positions: [
                tf_core::NormalizedRect::new(0.35, 0.4, 0.05, 0.08),
                tf_core::NormalizedRect::new(0.42, 0.4, 0.05, 0.08),
                tf_core::NormalizedRect::new(0.49, 0.4, 0.05, 0.08),
                tf_core::NormalizedRect::new(0.56, 0.4, 0.05, 0.08),
                tf_core::NormalizedRect::new(0.63, 0.4, 0.05, 0.08),
            ],
            pot_position: tf_core::NormalizedRect::new(0.45, 0.3, 0.1, 0.04),
            seat_positions: vec![],
            dealer_button_region: tf_core::NormalizedRect::new(0.3, 0.35, 0.03, 0.03),
            action_button_regions: [
                tf_core::NormalizedRect::new(0.4, 0.85, 0.08, 0.04),
                tf_core::NormalizedRect::new(0.5, 0.85, 0.08, 0.04),
                tf_core::NormalizedRect::new(0.6, 0.85, 0.08, 0.04),
                tf_core::NormalizedRect::new(0.7, 0.85, 0.08, 0.04),
            ],
            hero_seat: None,
            blinds: tf_core::BlindsInfo::default(),
            digit_ocr_regions: tf_core::DigitOcrRegions::default(),
            theme_id: "default".to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bridge::NapiBridge;
    use std::sync::Arc;
    use tf_core::{
        BlindsInfo, CaptureBackend, DigitOcrRegions, InferenceConfig, ManagerConfig,
        NormalizedRect, SeatCalibration, SeatId, TableCalibration, ThreadConfig,
    };
    use tf_table::TableManager;

    fn lock_test() -> std::sync::MutexGuard<'static, ()> {
        crate::bridge::TEST_LOCK.lock().unwrap()
    }

    fn setup_bridge() {
        NapiBridge::reset();
        let mgr = Arc::new(
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
        );
        NapiBridge::init(mgr).unwrap();
    }

    fn make_calibration() -> TableCalibration {
        TableCalibration {
            resolution: (1920, 1080),
            hole_card_positions: [
                tf_core::NormalizedRect::new(0.1, 0.5, 0.05, 0.08),
                tf_core::NormalizedRect::new(0.17, 0.5, 0.05, 0.08),
            ],
            community_card_positions: [
                tf_core::NormalizedRect::new(0.35, 0.4, 0.05, 0.08),
                tf_core::NormalizedRect::new(0.42, 0.4, 0.05, 0.08),
                tf_core::NormalizedRect::new(0.49, 0.4, 0.05, 0.08),
                tf_core::NormalizedRect::new(0.56, 0.4, 0.05, 0.08),
                tf_core::NormalizedRect::new(0.63, 0.4, 0.05, 0.08),
            ],
            pot_position: tf_core::NormalizedRect::new(0.45, 0.3, 0.1, 0.04),
            seat_positions: vec![tf_core::SeatCalibration {
                seat_id: SeatId::new(0),
                seat_region: tf_core::NormalizedRect::new(0.0, 0.0, 0.1, 0.1),
                stack_region: tf_core::NormalizedRect::new(0.0, 0.08, 0.1, 0.03),
                bet_region: tf_core::NormalizedRect::new(0.05, 0.12, 0.05, 0.03),
                avatar_region: tf_core::NormalizedRect::new(0.0, 0.0, 0.05, 0.05),
                card_region: None,
            }],
            dealer_button_region: tf_core::NormalizedRect::new(0.3, 0.35, 0.03, 0.03),
            action_button_regions: [
                tf_core::NormalizedRect::new(0.4, 0.85, 0.08, 0.04),
                tf_core::NormalizedRect::new(0.5, 0.85, 0.08, 0.04),
                tf_core::NormalizedRect::new(0.6, 0.85, 0.08, 0.04),
                tf_core::NormalizedRect::new(0.7, 0.85, 0.08, 0.04),
            ],
            hero_seat: Some(SeatId::new(0)),
            blinds: BlindsInfo::default(),
            digit_ocr_regions: tf_core::DigitOcrRegions::default(),
            theme_id: "test".to_string(),
        }
    }

    #[tokio::test]
    async fn test_start_capture_missing_table_id() {
        let _guard = lock_test();
        setup_bridge();
        let result = start_capture(serde_json::json!({})).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_start_and_stop_capture() {
        let _guard = lock_test();
        setup_bridge();
        let cal = make_calibration();
        let cal_json = serde_json::to_value(&cal).unwrap();
        start_capture(serde_json::json!({
            "tableId": "t1",
            "calibration": cal_json
        }))
        .await
        .unwrap();
        stop_capture("t1".to_string()).await.unwrap();
    }

    #[tokio::test]
    async fn test_discover_tables() {
        let result = discover_tables().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_get_table_state_not_found() {
        let _guard = lock_test();
        setup_bridge();
        let result = get_table_state("nonexistent".to_string()).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_calibrate_table() {
        let result = calibrate_table("t1".to_string()).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_shutdown() {
        let _guard = lock_test();
        setup_bridge();
        shutdown().await.unwrap();
    }
}
