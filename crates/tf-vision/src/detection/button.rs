//! 行动按钮检测（Fold / Call / Raise / All-in）
//!
//! 仅用于"判断当前是否轮到 hero 行动"，**不**用于推导动作语义。
//! 动作推导仍走 stack/pot diff。
//!
//! 检测逻辑：
//! - visible: 按钮区域亮度高于背景阈值（按钮被渲染）
//! - enabled: 按钮区域色彩饱和度足够高（非灰色 disabled 状态）

use tf_core::{Frame, PixelFormat, Rect, TfError};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActionButtonKind {
    Fold,
    Check,
    Call,
    Raise,
    AllIn,
}

#[derive(Debug, Clone)]
pub struct ActionButtonState {
    pub kind: ActionButtonKind,
    pub visible: bool,
    pub enabled: bool,
}

const VISIBILITY_BRIGHTNESS: f64 = 100.0;
const ENABLED_SATURATION: f64 = 30.0;

pub struct ActionButtonDetector;

impl ActionButtonDetector {
    pub fn detect(
        &self,
        action_button_rois: &[Rect; 4],
        frame: &Frame,
    ) -> Result<Vec<ActionButtonState>, TfError> {
        let kinds = [
            ActionButtonKind::Fold,
            ActionButtonKind::Check,
            ActionButtonKind::Call,
            ActionButtonKind::Raise,
        ];

        let mut states = Vec::with_capacity(4);
        for (i, kind) in kinds.into_iter().enumerate() {
            let roi = &action_button_rois[i];
            let roi_img = crop_roi(frame, roi);
            let (avg_brightness, avg_saturation) = brightness_and_saturation(&roi_img);

            let visible = avg_brightness > VISIBILITY_BRIGHTNESS;
            let enabled = visible && avg_saturation > ENABLED_SATURATION;

            states.push(ActionButtonState {
                kind,
                visible,
                enabled,
            });
        }

        Ok(states)
    }
}

fn brightness_and_saturation(roi: &Frame) -> (f64, f64) {
    if roi.data.is_empty() || roi.width == 0 || roi.height == 0 {
        return (0.0, 0.0);
    }

    let channels = match roi.format {
        PixelFormat::Gray8 => 1,
        PixelFormat::Bgr8 | PixelFormat::Rgb8 => 3,
        PixelFormat::Bgra8 => 4,
    };
    let pixel_count = roi.width as usize * roi.height as usize;
    let sample_step = (pixel_count / 64).max(1);
    let mut brightness_sum: f64 = 0.0;
    let mut saturation_sum: f64 = 0.0;
    let mut sample_count: f64 = 0.0;

    for i in (0..pixel_count).step_by(sample_step) {
        let off = i * channels;
        if off + 2 < roi.data.len() {
            let r = roi.data[off] as f64;
            let g = roi.data[off + 1] as f64;
            let b = roi.data[off + 2] as f64;

            brightness_sum += 0.299 * r + 0.587 * g + 0.114 * b;

            let max_val = r.max(g).max(b);
            let min_val = r.min(g).min(b);
            let sat = if max_val > 0.0 {
                (max_val - min_val) / max_val * 255.0
            } else {
                0.0
            };
            saturation_sum += sat;
            sample_count += 1.0;
        }
    }

    if sample_count == 0.0 {
        return (0.0, 0.0);
    }

    (
        brightness_sum / sample_count,
        saturation_sum / sample_count,
    )
}

fn crop_roi(frame: &Frame, roi: &Rect) -> Frame {
    let x0 = roi.x.max(0) as usize;
    let y0 = roi.y.max(0) as usize;
    let x1 = ((roi.x as usize) + (roi.width as usize)).min(frame.width as usize);
    let y1 = ((roi.y as usize) + (roi.height as usize)).min(frame.height as usize);
    let w = x1.saturating_sub(x0);
    let h = y1.saturating_sub(y0);
    let channels = match frame.format {
        PixelFormat::Gray8 => 1,
        PixelFormat::Bgr8 | PixelFormat::Rgb8 => 3,
        PixelFormat::Bgra8 => 4,
    };
    let stride = frame.stride as usize;
    let mut data = Vec::with_capacity(w * h * channels);
    for y in y0..y1 {
        let row_start = y * stride + x0 * channels;
        let row_end = y * stride + x1 * channels;
        if row_end <= frame.data.len() {
            data.extend_from_slice(&frame.data[row_start..row_end]);
        }
    }
    Frame {
        width: w as u32,
        height: h as u32,
        stride: (w * channels) as u32,
        format: frame.format,
        data: std::sync::Arc::new(data),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    fn make_frame(w: u32, h: u32, r: u8, g: u8, b: u8) -> Frame {
        let pixel_count = (w * h) as usize;
        let mut data = Vec::with_capacity(pixel_count * 3);
        for _ in 0..pixel_count {
            data.push(r);
            data.push(g);
            data.push(b);
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
    fn test_visible_enabled_colored_buttons() {
        let det = ActionButtonDetector;
        let frame = make_frame(640, 480, 100, 180, 80);
        let rois = [
            Rect::new(400, 400, 60, 30),
            Rect::new(470, 400, 60, 30),
            Rect::new(540, 400, 60, 30),
            Rect::new(610, 400, 60, 30),
        ];
        let states = det.detect(&rois, &frame).unwrap();
        assert_eq!(states.len(), 4);
        for s in &states {
            assert!(s.visible);
            assert!(s.enabled);
        }
    }

    #[test]
    fn test_visible_disabled_gray_buttons() {
        let det = ActionButtonDetector;
        let frame = make_frame(640, 480, 140, 140, 140);
        let rois = [
            Rect::new(400, 400, 60, 30),
            Rect::new(470, 400, 60, 30),
            Rect::new(540, 400, 60, 30),
            Rect::new(610, 400, 60, 30),
        ];
        let states = det.detect(&rois, &frame).unwrap();
        for s in &states {
            assert!(s.visible);
            assert!(!s.enabled);
        }
    }

    #[test]
    fn test_invisible_dark_buttons() {
        let det = ActionButtonDetector;
        let frame = make_frame(640, 480, 30, 30, 30);
        let rois = [
            Rect::new(400, 400, 60, 30),
            Rect::new(470, 400, 60, 30),
            Rect::new(540, 400, 60, 30),
            Rect::new(610, 400, 60, 30),
        ];
        let states = det.detect(&rois, &frame).unwrap();
        for s in &states {
            assert!(!s.visible);
            assert!(!s.enabled);
        }
    }

    #[test]
    fn test_empty_frame() {
        let det = ActionButtonDetector;
        let frame = Frame {
            width: 0,
            height: 0,
            stride: 0,
            format: PixelFormat::Rgb8,
            data: Arc::new(vec![]),
        };
        let rois = [
            Rect::new(0, 0, 0, 0),
            Rect::new(0, 0, 0, 0),
            Rect::new(0, 0, 0, 0),
            Rect::new(0, 0, 0, 0),
        ];
        let states = det.detect(&rois, &frame).unwrap();
        for s in &states {
            assert!(!s.visible);
            assert!(!s.enabled);
        }
    }
}
