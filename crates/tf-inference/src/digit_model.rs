//! Digit OCR trait + 输入预处理签名（PaddleOCR digit-only 模型）。

use async_trait::async_trait;

use tf_core::{Frame, TfError};

use crate::session::{DigitInput, DigitOutput};

/// 数字 OCR 识别结果（已解析为浮点数）
#[derive(Debug, Clone)]
pub struct DigitPrediction {
    pub raw_text: String,
    pub value: f64,
    pub confidence: f32,
    pub raw: DigitOutput,
}

/// 抽象的数字识别器。
///
/// detail-impl 阶段：实现 `PaddleDigitRecognizer`，内部用 `InferencePool`。
/// 测试时可用 `MockDigitRecognizer` 直接返回固定 value。
#[async_trait]
pub trait DigitRecognizer: Send + Sync {
    /// 给一张数字 ROI 图，返回识别出的浮点值。
    /// 若文本不含合法数字（或置信度过低）返回 `None`。
    async fn recognize(&self, roi: &Frame) -> Result<Option<DigitPrediction>, TfError>;
}

/// 把 ROI Frame 转成 32×100 灰度二值张量
/// TODO(detail-impl): grayscale → otsu threshold → resize
pub fn frame_to_digit_input(_roi: &Frame) -> Result<DigitInput, TfError> {
    todo!("frame_to_digit_input — grayscale + binarize + resize to 32x100")
}

/// 把 OCR 识别出的字符序列解析成浮点数。
/// TODO(detail-impl): 处理千分位、$、K/M 后缀、负号等
pub fn parse_number_from_digits(_text: &str) -> Option<f64> {
    todo!("parse_number_from_digits — strip $, commas, K/M suffix, parse f64")
}
