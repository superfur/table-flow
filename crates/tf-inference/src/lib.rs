//! tf-inference — ONNX Session Pool / OCR Assistant
//!
//! 架构骨架阶段：仅暴露 trait 与 pool 类型签名，不依赖 ort / onnx。
//! detail-impl 阶段加入 `ort` crate 后填充。

pub mod card_model;
pub mod digit_model;
pub mod ocr;
pub mod prepost;
pub mod session;

pub use card_model::*;
pub use digit_model::*;
pub use ocr::*;
pub use prepost::*;
pub use session::*;
