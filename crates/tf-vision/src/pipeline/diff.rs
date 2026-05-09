//! Frame differencing —— 检测帧间是否有显著变化（用于跳过无变化的帧）

use tf_core::Frame;

#[derive(Debug, Clone)]
pub struct DiffConfig {
    /// 变化像素比例阈值（0.0–1.0）
    pub threshold: f64,
}

impl Default for DiffConfig {
    fn default() -> Self {
        Self { threshold: 0.005 }
    }
}

pub struct DiffDetector {
    pub config: DiffConfig,
}

impl DiffDetector {
    pub fn new(config: DiffConfig) -> Self {
        Self { config }
    }

    /// TODO(detail-impl):
    ///   - absdiff(prev, curr) → gray → threshold(30) → count_non_zero
    ///   - 比较占比 > config.threshold
    pub fn has_significant_change(&self, _prev: &Frame, _curr: &Frame) -> bool {
        todo!("DiffDetector::has_significant_change")
    }
}
