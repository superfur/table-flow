//! 全局错误类型。
//!
//! 各 crate 内部可定义自己的具体错误，但跨 crate 边界统一用 `TfError`。

use thiserror::Error;

#[derive(Debug, Error)]
pub enum TfError {
    #[error("capture error: {0}")]
    Capture(String),

    #[error("vision pipeline error: {0}")]
    Vision(String),

    #[error("inference error: {0}")]
    Inference(String),

    #[error("ocr error: {0}")]
    Ocr(String),

    #[error("state machine error: {0}")]
    StateMachine(String),

    #[error("recommendation engine error: {0}")]
    Recommendation(String),

    #[error("ipc / napi error: {0}")]
    Ipc(String),

    #[error("calibration error: {0}")]
    Calibration(String),

    #[error("window not found: {0}")]
    WindowNotFound(String),

    #[error("table not found: {0}")]
    TableNotFound(String),

    #[error("config error: {0}")]
    Config(String),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("serde error: {0}")]
    Serde(#[from] serde_json::Error),

    #[error("other: {0}")]
    Other(String),
}

pub type Result<T, E = TfError> = std::result::Result<T, E>;
