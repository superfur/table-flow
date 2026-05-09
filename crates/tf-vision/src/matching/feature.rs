//! 特征向量提取（用于轻量比对，例如座位状态分类）

use tf_core::{Frame, TfError};

/// 一个 Frame 的紧凑特征向量
#[derive(Debug, Clone)]
pub struct FeatureVector {
    pub data: Vec<f32>,
}

pub struct FeatureExtractor;

impl FeatureExtractor {
    /// TODO(detail-impl): HSV histogram / 梯度 / 简单 CNN backbone
    pub fn extract(&self, _frame: &Frame) -> Result<FeatureVector, TfError> {
        todo!("FeatureExtractor::extract")
    }

    /// 余弦相似度
    pub fn cosine_similarity(a: &FeatureVector, b: &FeatureVector) -> f32 {
        if a.data.len() != b.data.len() || a.data.is_empty() {
            return 0.0;
        }
        let dot: f32 = a.data.iter().zip(&b.data).map(|(x, y)| x * y).sum();
        let na: f32 = a.data.iter().map(|x| x * x).sum::<f32>().sqrt();
        let nb: f32 = b.data.iter().map(|x| x * x).sum::<f32>().sqrt();
        if na == 0.0 || nb == 0.0 {
            0.0
        } else {
            dot / (na * nb)
        }
    }
}
