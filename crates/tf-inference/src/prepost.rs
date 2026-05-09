//! 共用的张量预处理 / 后处理原语。
//!
//! 这一模块本身不依赖 ONNX，纯粹是字节级别的转换工具。
//! detail-impl 阶段填充实现。

use tf_core::{Frame, TfError};

/// BGRA → RGB 字节转换（输入：每像素 4 字节 BGRA，输出：每像素 3 字节 RGB）
/// TODO(detail-impl): 使用 SIMD 或 OpenCV cvtColor
pub fn bgra_to_rgb(_frame: &Frame) -> Result<Frame, TfError> {
    todo!("bgra_to_rgb")
}

/// 把任意 PixelFormat 的 Frame 转为灰度
/// TODO(detail-impl)
pub fn to_grayscale(_frame: &Frame) -> Result<Frame, TfError> {
    todo!("to_grayscale")
}

/// 双线性插值 resize
/// TODO(detail-impl)
pub fn resize(_frame: &Frame, _target_w: u32, _target_h: u32) -> Result<Frame, TfError> {
    todo!("resize")
}

/// CTC 贪心解码（用于 PaddleOCR 输出 → 字符序列）
/// TODO(detail-impl)
pub fn ctc_greedy_decode(_logits: &[f32], _num_classes: usize) -> (String, f32) {
    todo!("ctc_greedy_decode")
}
