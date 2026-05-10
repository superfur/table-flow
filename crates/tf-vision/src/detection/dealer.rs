//! 庄家按钮检测
//!
//! 通过亮度阈值检测庄家按钮是否存在于给定 ROI 中。
//! 按钮通常为白色/亮色圆形标记，亮度显著高于桌面背景。

use tf_core::{Frame, PixelFormat, Rect, SeatId, TfError};

use crate::pipeline::SeatRoi;

const BRIGHTNESS_THRESHOLD: f64 = 180.0;

pub struct DealerTracker {
    pub last_position: Option<SeatId>,
}

impl Default for DealerTracker {
    fn default() -> Self {
        Self {
            last_position: None,
        }
    }
}

impl DealerTracker {
    pub fn detect(
        &mut self,
        dealer_roi: &Rect,
        seat_rois: &[SeatRoi],
        frame: &Frame,
    ) -> Result<Option<SeatId>, TfError> {
        let roi_img = crop_roi(frame, dealer_roi);
        if !is_button_present(&roi_img) {
            return Ok(self.last_position);
        }

        let center_x = dealer_roi.x as f64 + dealer_roi.width as f64 / 2.0;
        let center_y = dealer_roi.y as f64 + dealer_roi.height as f64 / 2.0;

        let mut best_seat = None;
        let mut best_dist = f64::MAX;

        for seat_roi in seat_rois {
            let sx = seat_roi.seat_area.x as f64 + seat_roi.seat_area.width as f64 / 2.0;
            let sy = seat_roi.seat_area.y as f64 + seat_roi.seat_area.height as f64 / 2.0;
            let dist = (center_x - sx).powi(2) + (center_y - sy).powi(2);
            if dist < best_dist {
                best_dist = dist;
                best_seat = Some(seat_roi.seat_id);
            }
        }

        if let Some(seat) = best_seat {
            self.last_position = Some(seat);
        }

        Ok(best_seat.or(self.last_position))
    }
}

fn is_button_present(roi: &Frame) -> bool {
    if roi.data.is_empty() || roi.width == 0 || roi.height == 0 {
        return false;
    }
    let channels = match roi.format {
        PixelFormat::Gray8 => 1,
        PixelFormat::Bgr8 | PixelFormat::Rgb8 => 3,
        PixelFormat::Bgra8 => 4,
    };
    let pixel_count = roi.width as usize * roi.height as usize;
    let sample_step = (pixel_count / 64).max(1);
    let mut bright_count: f64 = 0.0;
    let mut total: f64 = 0.0;

    for i in (0..pixel_count).step_by(sample_step) {
        let off = i * channels;
        if off + channels <= roi.data.len() {
            let b = roi.data[off];
            let g = roi.data.get(off + 1).copied().unwrap_or(b);
            let r = roi.data.get(off + 2).copied().unwrap_or(b);
            let lum = 0.114 * b as f64 + 0.587 * g as f64 + 0.299 * r as f64;
            if lum > BRIGHTNESS_THRESHOLD {
                bright_count += 1.0;
            }
            total += 1.0;
        }
    }

    if total == 0.0 {
        return false;
    }

    bright_count / total > 0.4
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

    fn make_frame(w: u32, h: u32, fill: u8) -> Frame {
        Frame {
            width: w,
            height: h,
            stride: w * 3,
            format: PixelFormat::Rgb8,
            data: Arc::new(vec![fill; (w * h * 3) as usize]),
        }
    }

    fn make_seat_roi(id: u8, x: i32, y: i32) -> SeatRoi {
        SeatRoi {
            seat_id: SeatId::new(id),
            seat_area: Rect::new(x, y, 80, 80),
            stack_area: Rect::new(x, y + 60, 80, 20),
            bet_area: Rect::new(x + 40, y + 80, 40, 15),
            avatar_area: Rect::new(x, y, 40, 40),
            card_area: None,
        }
    }

    #[test]
    fn test_no_button_dark() {
        let mut tracker = DealerTracker::default();
        let frame = make_frame(640, 480, 30);
        let dealer_roi = Rect::new(100, 100, 30, 30);
        let seats = vec![make_seat_roi(0, 80, 80), make_seat_roi(1, 300, 80)];
        let result = tracker.detect(&dealer_roi, &seats, &frame).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_button_present_near_seat0() {
        let mut tracker = DealerTracker::default();
        let frame = make_frame(640, 480, 230);
        let dealer_roi = Rect::new(100, 90, 30, 30);
        let seats = vec![
            make_seat_roi(0, 80, 80),
            make_seat_roi(1, 400, 80),
        ];
        let result = tracker.detect(&dealer_roi, &seats, &frame).unwrap();
        assert_eq!(result, Some(SeatId::new(0)));
        assert_eq!(tracker.last_position, Some(SeatId::new(0)));
    }

    #[test]
    fn test_button_persists_when_not_present() {
        let mut tracker = DealerTracker::default();
        tracker.last_position = Some(SeatId::new(2));
        let frame = make_frame(640, 480, 30);
        let dealer_roi = Rect::new(100, 100, 30, 30);
        let seats = vec![make_seat_roi(0, 80, 80)];
        let result = tracker.detect(&dealer_roi, &seats, &frame).unwrap();
        assert_eq!(result, Some(SeatId::new(2)));
    }

    #[test]
    fn test_empty_roi() {
        let mut tracker = DealerTracker::default();
        let frame = Frame {
            width: 0,
            height: 0,
            stride: 0,
            format: PixelFormat::Rgb8,
            data: Arc::new(vec![]),
        };
        let dealer_roi = Rect::new(0, 0, 0, 0);
        let result = tracker.detect(&dealer_roi, &[], &frame).unwrap();
        assert!(result.is_none());
    }
}
