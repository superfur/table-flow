//! Preprocessor —— 帧标准化（resize / 颜色空间 / 去噪）

use tf_core::{Frame, TfError};

#[derive(Debug, Clone)]
pub struct PreprocessorConfig {
    pub target_size: Option<(u32, u32)>,
    pub denoise: bool,
}

impl Default for PreprocessorConfig {
    fn default() -> Self {
        Self {
            target_size: None,
            denoise: false,
        }
    }
}

pub struct Preprocessor {
    pub config: PreprocessorConfig,
}

impl Preprocessor {
    pub fn new(config: PreprocessorConfig) -> Self {
        Self { config }
    }

    /// TODO(detail-impl):
    ///   - 按 target_size resize
    ///   - BGRA → BGR（节省 25% 内存）
    ///   - 可选 fast NL means 去噪
    pub fn process(&self, _frame: &Frame) -> Result<Frame, TfError> {
        todo!("Preprocessor::process")
    }
}
