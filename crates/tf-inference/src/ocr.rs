//! OcrAssistant —— 视觉 pipeline 的 OCR 辅助层。
//!
//! 本层是 `tf-vision` 调用 OCR 的统一入口。架构上做两件事：
//!   1. 持有一个 `dyn DigitRecognizer`（默认 PaddleOCR backed by InferencePool）
//!   2. 提供高层 API（`recognize_digits / recognize_pot / recognize_stack`），
//!      隐藏预处理 / 后处理细节
//!
//! **关键约束**：OCR 在本系统中是 *辅助路径*，不是主推导路径。
//! 主推导（动作语义）依然由 `tf-state::ActionReconstructor` 通过 stack/pot diff 完成。

use std::sync::Arc;

use async_trait::async_trait;

use tf_core::{Frame, TfError};

use crate::digit_model::{DigitPrediction, DigitRecognizer};

/// OcrAssistant 的对外接口（trait 形式，便于测试 mock）
#[async_trait]
pub trait Ocr: Send + Sync {
    async fn recognize_digits(&self, roi: &Frame) -> Result<Option<f64>, TfError>;
}

/// 默认实现：包装一个 `DigitRecognizer`
pub struct OcrAssistant {
    recognizer: Arc<dyn DigitRecognizer>,
    enabled: bool,
}

impl OcrAssistant {
    pub fn new(recognizer: Arc<dyn DigitRecognizer>) -> Self {
        Self { recognizer, enabled: true }
    }

    pub fn disabled() -> Self {
        // 一个不持有 recognizer 的占位；调用 recognize_digits 总是返回 None
        Self {
            recognizer: Arc::new(NullRecognizer),
            enabled: false,
        }
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled
    }
}

#[async_trait]
impl Ocr for OcrAssistant {
    async fn recognize_digits(&self, roi: &Frame) -> Result<Option<f64>, TfError> {
        if !self.enabled {
            return Ok(None);
        }
        let pred: Option<DigitPrediction> = self.recognizer.recognize(roi).await?;
        Ok(pred.map(|p| p.value))
    }
}

// =============================================================================
// Null implementation for tests / disabled state
// =============================================================================

struct NullRecognizer;

#[async_trait]
impl DigitRecognizer for NullRecognizer {
    async fn recognize(&self, _roi: &Frame) -> Result<Option<DigitPrediction>, TfError> {
        Ok(None)
    }
}
