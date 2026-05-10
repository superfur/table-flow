//! Frame differencing —— 检测帧间是否有显著变化

use tf_core::{Frame, PixelFormat};

#[derive(Debug, Clone)]
pub struct DiffConfig {
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

    pub fn has_significant_change(&self, prev: &Frame, curr: &Frame) -> bool {
        if prev.width != curr.width || prev.height != curr.height {
            return true;
        }
        if prev.format != curr.format {
            return true;
        }

        let bytes_per_pixel = match prev.format {
            PixelFormat::Gray8 => 1,
            PixelFormat::Bgr8 | PixelFormat::Rgb8 => 3,
            PixelFormat::Bgra8 => 4,
        };
        let pixel_count = prev.width as usize * prev.height as usize;
        let total_bytes = pixel_count * bytes_per_pixel;

        let prev_data = &prev.data;
        let curr_data = &curr.data;
        let len = prev_data.len().min(curr_data.len()).min(total_bytes);

        let mut diff_count: usize = 0;
        const DIFF_THRESHOLD: u8 = 30;

        for i in 0..len {
            let d = if prev_data[i] > curr_data[i] {
                prev_data[i] - curr_data[i]
            } else {
                curr_data[i] - prev_data[i]
            };
            if d > DIFF_THRESHOLD {
                diff_count += 1;
            }
        }

        let ratio = diff_count as f64 / pixel_count as f64;
        ratio > self.config.threshold
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
    fn test_same_frame_no_change() {
        let det = DiffDetector::new(DiffConfig::default());
        let f = make_frame(100, 100, 128);
        assert!(!det.has_significant_change(&f, &f));
    }

    #[test]
    fn test_different_frames() {
        let det = DiffDetector::new(DiffConfig::default());
        let f1 = make_frame(100, 100, 0);
        let f2 = make_frame(100, 100, 200);
        assert!(det.has_significant_change(&f1, &f2));
    }

    #[test]
    fn test_slightly_different() {
        let det = DiffDetector::new(DiffConfig { threshold: 0.1 });
        let f1 = make_frame(100, 100, 100);
        let mut data = vec![100u8; 30000];
        data[0] = 101;
        let f2 = Frame {
            width: 100, height: 100, stride: 300,
            format: PixelFormat::Rgb8,
            data: Arc::new(data),
        };
        assert!(!det.has_significant_change(&f1, &f2));
    }

    #[test]
    fn test_size_mismatch() {
        let det = DiffDetector::new(DiffConfig::default());
        let f1 = make_frame(100, 100, 128);
        let f2 = make_frame(200, 200, 128);
        assert!(det.has_significant_change(&f1, &f2));
    }
}
