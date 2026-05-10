//! Preprocessor —— 帧标准化

use tf_core::{Frame, TfError};
use tf_inference::prepost;

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

    pub fn process(&self, frame: &Frame) -> Result<Frame, TfError> {
        let mut result = frame.clone();

        if let Some((tw, th)) = self.config.target_size {
            if frame.width != tw || frame.height != th {
                result = prepost::resize(frame, tw, th)?;
            }
        }

        if self.config.denoise {
            // Simple box-blur approximation for denoising (no OpenCV dependency)
            result = box_blur_3x3(&result);
        }

        Ok(result)
    }
}

fn box_blur_3x3(frame: &Frame) -> Frame {
    let channels = match frame.format {
        tf_core::PixelFormat::Gray8 => 1,
        tf_core::PixelFormat::Bgr8 | tf_core::PixelFormat::Rgb8 => 3,
        tf_core::PixelFormat::Bgra8 => 4,
    };
    let w = frame.width as usize;
    let h = frame.height as usize;
    let src = &frame.data;
    let mut dst = vec![0u8; src.len()];

    for y in 0..h {
        for x in 0..w {
            let mut sum = vec![0u32; channels];
            let mut count = 0u32;
            for dy in -1i32..=1 {
                for dx in -1i32..=1 {
                    let ny = (y as i32 + dy).clamp(0, h as i32 - 1) as usize;
                    let nx = (x as i32 + dx).clamp(0, w as i32 - 1) as usize;
                    let off = (ny * w + nx) * channels;
                    for c in 0..channels {
                        sum[c] += src[off + c] as u32;
                    }
                    count += 1;
                }
            }
            let off = (y * w + x) * channels;
            for c in 0..channels {
                dst[off + c] = (sum[c] / count) as u8;
            }
        }
    }

    Frame {
        width: frame.width,
        height: frame.height,
        stride: frame.stride,
        format: frame.format,
        data: std::sync::Arc::new(dst),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    fn make_frame(w: u32, h: u32) -> Frame {
        Frame {
            width: w,
            height: h,
            stride: w * 3,
            format: tf_core::PixelFormat::Rgb8,
            data: Arc::new(vec![128u8; (w * h * 3) as usize]),
        }
    }

    #[test]
    fn test_no_resize() {
        let pre = Preprocessor::new(PreprocessorConfig::default());
        let frame = make_frame(100, 100);
        let result = pre.process(&frame).unwrap();
        assert_eq!(result.width, 100);
    }

    #[test]
    fn test_resize() {
        let pre = Preprocessor::new(PreprocessorConfig {
            target_size: Some((50, 50)),
            denoise: false,
        });
        let frame = make_frame(100, 100);
        let result = pre.process(&frame).unwrap();
        assert_eq!(result.width, 50);
        assert_eq!(result.height, 50);
    }

    #[test]
    fn test_denoise() {
        let pre = Preprocessor::new(PreprocessorConfig {
            target_size: None,
            denoise: true,
        });
        let frame = make_frame(10, 10);
        let result = pre.process(&frame).unwrap();
        assert_eq!(result.width, 10);
    }
}
