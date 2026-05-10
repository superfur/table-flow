//! Digit OCR trait + 输入预处理签名（PaddleOCR digit-only 模型）。

use async_trait::async_trait;

use tf_core::{Frame, TfError};

use crate::prepost;
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
#[async_trait]
pub trait DigitRecognizer: Send + Sync {
    async fn recognize(&self, roi: &Frame) -> Result<Option<DigitPrediction>, TfError>;
}

const DIGIT_MODEL_WIDTH: u32 = 100;
const DIGIT_MODEL_HEIGHT: u32 = 32;

/// 把 ROI Frame 转成 32×100 灰度张量
pub fn frame_to_digit_input(roi: &Frame) -> Result<DigitInput, TfError> {
    let gray = prepost::to_grayscale(roi)?;
    let resized = prepost::resize(&gray, DIGIT_MODEL_WIDTH, DIGIT_MODEL_HEIGHT)?;
    Ok(DigitInput {
        data: resized.data.clone(),
        width: DIGIT_MODEL_WIDTH,
        height: DIGIT_MODEL_HEIGHT,
    })
}

/// 把 OCR 识别出的字符序列解析成浮点数。
pub fn parse_number_from_digits(text: &str) -> Option<f64> {
    let cleaned: String = text
        .trim()
        .replace('$', "")
        .replace('€', "")
        .replace('£', "")
        .replace(',', "")
        .replace(' ', "");

    if cleaned.is_empty() {
        return None;
    }

    // Handle K/M suffixes
    let (num_part, multiplier) = if let Some(rest) = cleaned.strip_suffix('K') {
        (rest, 1_000.0)
    } else if let Some(rest) = cleaned.strip_suffix('M') {
        (rest, 1_000_000.0)
    } else if let Some(rest) = cleaned.strip_suffix('k') {
        (rest, 1_000.0)
    } else if let Some(rest) = cleaned.strip_suffix('m') {
        (rest, 1_000_000.0)
    } else {
        (cleaned.as_str(), 1.0)
    };

    let parsed: f64 = num_part.parse().ok()?;
    let result = parsed * multiplier;

    if result.is_finite() && result >= 0.0 {
        Some(result)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_number() {
        assert_eq!(parse_number_from_digits("123"), Some(123.0));
    }

    #[test]
    fn test_parse_decimal() {
        assert_eq!(parse_number_from_digits("12.50"), Some(12.5));
    }

    #[test]
    fn test_parse_with_dollar_sign() {
        assert_eq!(parse_number_from_digits("$500"), Some(500.0));
    }

    #[test]
    fn test_parse_with_commas() {
        assert_eq!(parse_number_from_digits("1,234.56"), Some(1234.56));
    }

    #[test]
    fn test_parse_with_k_suffix() {
        assert_eq!(parse_number_from_digits("1.5K"), Some(1500.0));
    }

    #[test]
    fn test_parse_with_m_suffix() {
        assert_eq!(parse_number_from_digits("2.5M"), Some(2_500_000.0));
    }

    #[test]
    fn test_parse_with_spaces() {
        assert_eq!(parse_number_from_digits(" 42 "), Some(42.0));
    }

    #[test]
    fn test_parse_empty() {
        assert_eq!(parse_number_from_digits(""), None);
    }

    #[test]
    fn test_parse_invalid() {
        assert_eq!(parse_number_from_digits("abc"), None);
    }

    #[test]
    fn test_parse_zero() {
        assert_eq!(parse_number_from_digits("0"), Some(0.0));
    }

    #[test]
    fn test_parse_dollar_commas() {
        assert_eq!(parse_number_from_digits("$10,000"), Some(10_000.0));
    }

    #[test]
    fn test_frame_to_digit_input_dimensions() {
        use tf_core::PixelFormat;
        let data = vec![128u8; 200 * 100 * 3];
        let frame = Frame {
            width: 200,
            height: 100,
            stride: 600,
            format: PixelFormat::Rgb8,
            data: std::sync::Arc::new(data),
        };
        let input = frame_to_digit_input(&frame).unwrap();
        assert_eq!(input.width, 100);
        assert_eq!(input.height, 32);
        assert_eq!(input.data.len(), 100 * 32);
    }
}
