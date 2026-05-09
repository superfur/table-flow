//! tf-vision —— 视觉 pipeline（capture / preprocess / detection / aggregation）
//!
//! 模块边界：
//! ```text
//! capture::FrameCapture  ──▶ pipeline::Preprocessor ──▶ pipeline::RoiManager
//!                                                            │
//!                                                            ▼
//!                                                     detection::*
//!                                                            │
//!                                                            ▼
//!                                                pipeline::FeatureAggregator
//!                                                            │
//!                                                  flume::Sender<ExtractedFeatures>
//! ```
//!
//! 公开 API 不依赖任何具体 CV 库；detail-impl 阶段在各模块内部使用 OpenCV。

pub mod calibration;
pub mod capture;
pub mod detection;
pub mod features;
pub mod matching;
pub mod pipeline;

pub use calibration::*;
pub use capture::*;
pub use detection::*;
pub use features::*;
pub use matching::*;
pub use pipeline::*;
