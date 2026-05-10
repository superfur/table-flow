//! 轮廓分析（用于 auto-calibration、卡片定位）
//!
//! MVP 实现：基于亮度阈值的连通区域检测。
//! 通过二值化后扫描相邻亮像素群来提取轮廓边界框。

use tf_core::{Frame, PixelFormat, Rect, TfError};

#[derive(Debug, Clone)]
pub struct Contour {
    pub bounding_box: Rect,
    pub aspect_ratio: f32,
    pub area: u32,
}

const BINARY_THRESHOLD: u8 = 128;
const MIN_CONTOUR_AREA: u32 = 100;

pub struct ContourAnalyzer {
    pub threshold: u8,
    pub min_area: u32,
}

impl Default for ContourAnalyzer {
    fn default() -> Self {
        Self {
            threshold: BINARY_THRESHOLD,
            min_area: MIN_CONTOUR_AREA,
        }
    }
}

impl ContourAnalyzer {
    pub fn new(threshold: u8, min_area: u32) -> Self {
        Self { threshold, min_area }
    }

    pub fn find_all(&self, frame: &Frame) -> Result<Vec<Contour>, TfError> {
        if frame.width == 0 || frame.height == 0 || frame.data.is_empty() {
            return Ok(Vec::new());
        }

        let w = frame.width as usize;
        let h = frame.height as usize;
        let channels = match frame.format {
            PixelFormat::Gray8 => 1,
            PixelFormat::Bgr8 | PixelFormat::Rgb8 => 3,
            PixelFormat::Bgra8 => 4,
        };
        let stride = frame.stride as usize;

        let mut binary = vec![false; w * h];
        for y in 0..h {
            for x in 0..w {
                let off = y * stride + x * channels;
                if off < frame.data.len() {
                    let lum = if channels >= 3 {
                        0.299 * frame.data[off] as f32
                            + 0.587 * frame.data[off + 1] as f32
                            + 0.114 * frame.data[off + 2] as f32
                    } else {
                        frame.data[off] as f32
                    };
                    binary[y * w + x] = lum >= self.threshold as f32;
                }
            }
        }

        let mut labels = vec![0u32; w * h];
        let mut next_label: u32 = 1;
        let mut contours: Vec<Contour> = Vec::new();

        for y in 0..h {
            for x in 0..w {
                if binary[y * w + x] && labels[y * w + x] == 0 {
                    let label = next_label;
                    next_label += 1;

                    let mut x0 = x;
                    let mut y0 = y;
                    let mut x1 = x;
                    let mut y1 = y;
                    let mut area: u32 = 0;

                    let mut stack = vec![(x, y)];
                    while let Some((cx, cy)) = stack.pop() {
                        if cx >= w || cy >= h {
                            continue;
                        }
                        let idx = cy * w + cx;
                        if !binary[idx] || labels[idx] != 0 {
                            continue;
                        }
                        labels[idx] = label;
                        area += 1;

                        if cx < x0 { x0 = cx; }
                        if cy < y0 { y0 = cy; }
                        if cx > x1 { x1 = cx; }
                        if cy > y1 { y1 = cy; }

                        if cx > 0 { stack.push((cx - 1, cy)); }
                        if cx + 1 < w { stack.push((cx + 1, cy)); }
                        if cy > 0 { stack.push((cx, cy - 1)); }
                        if cy + 1 < h { stack.push((cx, cy + 1)); }
                    }

                    if area >= self.min_area {
                        let bw = (x1 - x0 + 1) as u32;
                        let bh = (y1 - y0 + 1) as u32;
                        let ar = if bh > 0 {
                            bw as f32 / bh as f32
                        } else {
                            0.0
                        };
                        contours.push(Contour {
                            bounding_box: Rect::new(x0 as i32, y0 as i32, bw, bh),
                            aspect_ratio: ar,
                            area,
                        });
                    }
                }
            }
        }

        Ok(contours)
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

    fn make_frame_with_rect(
        w: u32,
        h: u32,
        rx: usize,
        ry: usize,
        rw: usize,
        rh: usize,
        fg: u8,
        bg: u8,
    ) -> Frame {
        let mut data = vec![bg; (w * h * 3) as usize];
        for y in ry..(ry + rh) {
            for x in rx..(rx + rw) {
                let off = (y * w as usize + x) * 3;
                if off + 2 < data.len() {
                    data[off] = fg;
                    data[off + 1] = fg;
                    data[off + 2] = fg;
                }
            }
        }
        Frame {
            width: w,
            height: h,
            stride: w * 3,
            format: PixelFormat::Rgb8,
            data: Arc::new(data),
        }
    }

    #[test]
    fn test_no_contours_dark() {
        let analyzer = ContourAnalyzer::default();
        let frame = make_frame(100, 100, 30);
        let contours = analyzer.find_all(&frame).unwrap();
        assert!(contours.is_empty());
    }

    #[test]
    fn test_single_bright_region() {
        let analyzer = ContourAnalyzer::new(128, 10);
        let frame = make_frame_with_rect(200, 200, 50, 50, 40, 30, 220, 30);
        let contours = analyzer.find_all(&frame).unwrap();
        assert_eq!(contours.len(), 1);
        assert_eq!(contours[0].bounding_box.x, 50);
        assert_eq!(contours[0].bounding_box.y, 50);
        assert_eq!(contours[0].bounding_box.width, 40);
        assert_eq!(contours[0].bounding_box.height, 30);
        assert!(contours[0].area >= 10);
    }

    #[test]
    fn test_two_separate_regions() {
        let analyzer = ContourAnalyzer::new(128, 10);
        let mut data = vec![30u8; (200 * 200 * 3) as usize];
        for y in 20..50 {
            for x in 10..40 {
                let off = (y * 200 + x) * 3;
                data[off] = 220; data[off+1] = 220; data[off+2] = 220;
            }
        }
        for y in 100..140 {
            for x in 120..180 {
                let off = (y * 200 + x) * 3;
                data[off] = 220; data[off+1] = 220; data[off+2] = 220;
            }
        }
        let frame = Frame {
            width: 200, height: 200, stride: 200 * 3,
            format: PixelFormat::Rgb8,
            data: Arc::new(data),
        };
        let contours = analyzer.find_all(&frame).unwrap();
        assert_eq!(contours.len(), 2);
    }

    #[test]
    fn test_empty_frame() {
        let analyzer = ContourAnalyzer::default();
        let frame = Frame {
            width: 0, height: 0, stride: 0,
            format: PixelFormat::Rgb8,
            data: Arc::new(vec![]),
        };
        let contours = analyzer.find_all(&frame).unwrap();
        assert!(contours.is_empty());
    }

    #[test]
    fn test_small_region_filtered() {
        let analyzer = ContourAnalyzer::new(128, 1000);
        let frame = make_frame_with_rect(200, 200, 50, 50, 10, 10, 220, 30);
        let contours = analyzer.find_all(&frame).unwrap();
        assert!(contours.is_empty());
    }
}
