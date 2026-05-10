//! 特征向量提取（用于轻量比对，例如座位状态分类）
//!
//! MVP 实现：基于亮度直方图的特征向量（16-bin）。
//! 将 ROI 的灰度亮度映射到 16 个 bin，归一化后作为特征向量。

use tf_core::{Frame, PixelFormat, TfError};

const HIST_BINS: usize = 16;

#[derive(Debug, Clone)]
pub struct FeatureVector {
    pub data: Vec<f32>,
}

pub struct FeatureExtractor;

impl FeatureExtractor {
    pub fn extract(&self, frame: &Frame) -> Result<FeatureVector, TfError> {
        if frame.width == 0 || frame.height == 0 || frame.data.is_empty() {
            return Ok(FeatureVector {
                data: vec![0.0; HIST_BINS],
            });
        }

        let channels = match frame.format {
            PixelFormat::Gray8 => 1,
            PixelFormat::Bgr8 | PixelFormat::Rgb8 => 3,
            PixelFormat::Bgra8 => 4,
        };
        let _pixel_count = frame.width as usize * frame.height as usize;
        let stride = frame.stride as usize;

        let mut hist = vec![0f32; HIST_BINS];

        for y in 0..(frame.height as usize) {
            for x in 0..(frame.width as usize) {
                let off = y * stride + x * channels;
                if off + channels <= frame.data.len() {
                    let lum = if channels >= 3 {
                        0.299 * frame.data[off] as f32
                            + 0.587 * frame.data[off + 1] as f32
                            + 0.114 * frame.data[off + 2] as f32
                    } else {
                        frame.data[off] as f32
                    };
                    let bin = (lum / 256.0 * HIST_BINS as f32).min((HIST_BINS - 1) as f32) as usize;
                    hist[bin] += 1.0;
                }
            }
        }

        let total: f32 = hist.iter().sum();
        if total > 0.0 {
            for v in &mut hist {
                *v /= total;
            }
        }

        Ok(FeatureVector { data: hist })
    }

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

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    fn make_frame(w: u32, h: u32, fill: u8) -> Frame {
        Frame {
            width: w,
            height: h,
            stride: w * 3,
            format: PixelFormat::Rgb8,
            data: Arc::new(vec![fill; (w * h * 3) as usize]),
        }
    }

    #[test]
    fn test_extract_returns_16_bins() {
        let ext = FeatureExtractor;
        let frame = make_frame(64, 64, 128);
        let fv = ext.extract(&frame).unwrap();
        assert_eq!(fv.data.len(), 16);
    }

    #[test]
    fn test_extract_normalized() {
        let ext = FeatureExtractor;
        let frame = make_frame(64, 64, 128);
        let fv = ext.extract(&frame).unwrap();
        let sum: f32 = fv.data.iter().sum();
        assert!((sum - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_extract_empty_frame() {
        let ext = FeatureExtractor;
        let frame = Frame {
            width: 0,
            height: 0,
            stride: 0,
            format: PixelFormat::Rgb8,
            data: Arc::new(vec![]),
        };
        let fv = ext.extract(&frame).unwrap();
        assert_eq!(fv.data.len(), 16);
        assert!(fv.data.iter().all(|&v| v == 0.0));
    }

    #[test]
    fn test_cosine_similarity_identical() {
        let a = FeatureVector {
            data: vec![0.1, 0.2, 0.3, 0.4],
        };
        let sim = FeatureExtractor::cosine_similarity(&a, &a);
        assert!((sim - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_cosine_similarity_orthogonal() {
        let a = FeatureVector {
            data: vec![1.0, 0.0],
        };
        let b = FeatureVector {
            data: vec![0.0, 1.0],
        };
        let sim = FeatureExtractor::cosine_similarity(&a, &b);
        assert!((sim - 0.0).abs() < 0.001);
    }

    #[test]
    fn test_cosine_similarity_different_lengths() {
        let a = FeatureVector { data: vec![1.0] };
        let b = FeatureVector { data: vec![1.0, 2.0] };
        let sim = FeatureExtractor::cosine_similarity(&a, &b);
        assert_eq!(sim, 0.0);
    }

    #[test]
    fn test_bright_frame_hits_high_bin() {
        let ext = FeatureExtractor;
        let frame = make_frame(64, 64, 240);
        let fv = ext.extract(&frame).unwrap();
        let high_sum: f32 = fv.data[12..].iter().sum();
        assert!(high_sum > 0.9);
    }
}
