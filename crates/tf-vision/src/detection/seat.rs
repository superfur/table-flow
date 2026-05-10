//! 座位状态追踪

use std::collections::HashMap;
use std::time::Instant;

use tf_core::{Frame, PixelFormat, Rect, SeatId, SeatStatus, TfError};

use crate::features::SeatChange;
use crate::pipeline::SeatRoi;

#[derive(Debug, Clone)]
pub struct TrackedSeat {
    pub seat_id: SeatId,
    pub status: SeatStatus,
    pub last_seen_active: Instant,
}

pub struct SeatTracker {
    pub seat_states: HashMap<SeatId, TrackedSeat>,
}

impl Default for SeatTracker {
    fn default() -> Self {
        Self {
            seat_states: HashMap::new(),
        }
    }
}

impl SeatTracker {
    pub fn track(
        &mut self,
        seat_rois: &[SeatRoi],
        frame: &Frame,
    ) -> Result<Vec<SeatChange>, TfError> {
        let mut changes = Vec::new();

        for seat_roi in seat_rois {
            let seat_img = crop_roi(frame, &seat_roi.seat_area);
            let current_status = self.classify_seat_status(&seat_img)?;

            let prev = self.seat_states.get(&seat_roi.seat_id);
            let prev_status = prev.map(|p| p.status.clone());

            if prev.map_or(true, |p| p.status != current_status) {
                changes.push(SeatChange {
                    seat_id: seat_roi.seat_id,
                    prev_status,
                    new_status: current_status.clone(),
                });
            }

            let last_seen = if matches!(current_status, SeatStatus::Active) {
                Instant::now()
            } else {
                prev.map(|p| p.last_seen_active).unwrap_or_else(Instant::now)
            };

            self.seat_states.insert(
                seat_roi.seat_id,
                TrackedSeat {
                    seat_id: seat_roi.seat_id,
                    status: current_status,
                    last_seen_active: last_seen,
                },
            );
        }

        Ok(changes)
    }

    pub fn classify_seat_status(&self, seat_img: &Frame) -> Result<SeatStatus, TfError> {
        let channels = match seat_img.format {
            PixelFormat::Gray8 => 1,
            PixelFormat::Bgr8 | PixelFormat::Rgb8 => 3,
            PixelFormat::Bgra8 => 4,
        };
        let pixel_count = seat_img.width as usize * seat_img.height as usize;
        if pixel_count == 0 || seat_img.data.is_empty() {
            return Ok(SeatStatus::Empty);
        }

        let sample_step = (pixel_count / 100).max(1);
        let mut brightness_sum: f64 = 0.0;
        let mut sample_count: f64 = 0.0;

        for i in (0..pixel_count).step_by(sample_step) {
            let off = i * channels;
            if off + channels <= seat_img.data.len() {
                let b = seat_img.data[off];
                let g = seat_img.data.get(off + 1).copied().unwrap_or(b);
                let r = seat_img.data.get(off + 2).copied().unwrap_or(b);
                brightness_sum += 0.114 * b as f64 + 0.587 * g as f64 + 0.299 * r as f64;
                sample_count += 1.0;
            }
        }

        let avg = if sample_count > 0.0 {
            brightness_sum / sample_count
        } else {
            0.0
        };

        let status = if avg < 40.0 {
            SeatStatus::Empty
        } else if avg < 80.0 {
            SeatStatus::SittingOut
        } else if avg < 140.0 {
            SeatStatus::Folded
        } else {
            SeatStatus::Active
        };

        Ok(status)
    }
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
            width: w, height: h, stride: w * 3,
            format: PixelFormat::Rgb8,
            data: Arc::new(vec![fill; (w * h * 3) as usize]),
        }
    }

    #[test]
    fn test_classify_empty() {
        let tracker = SeatTracker::default();
        let frame = make_frame(50, 50, 20);
        let status = tracker.classify_seat_status(&frame).unwrap();
        assert_eq!(status, SeatStatus::Empty);
    }

    #[test]
    fn test_classify_active() {
        let tracker = SeatTracker::default();
        let frame = make_frame(50, 50, 180);
        let status = tracker.classify_seat_status(&frame).unwrap();
        assert_eq!(status, SeatStatus::Active);
    }

    #[test]
    fn test_classify_folded() {
        let tracker = SeatTracker::default();
        let frame = make_frame(50, 50, 100);
        let status = tracker.classify_seat_status(&frame).unwrap();
        assert_eq!(status, SeatStatus::Folded);
    }

    #[test]
    fn test_classify_empty_frame() {
        let tracker = SeatTracker::default();
        let frame = Frame {
            width: 0, height: 0, stride: 0,
            format: PixelFormat::Rgb8,
            data: Arc::new(vec![]),
        };
        let status = tracker.classify_seat_status(&frame).unwrap();
        assert_eq!(status, SeatStatus::Empty);
    }

    #[test]
    fn test_track_detects_change() {
        let mut tracker = SeatTracker::default();
        let frame = make_frame(640, 480, 180);
        let rois = vec![SeatRoi {
            seat_id: SeatId::new(0),
            seat_area: Rect::new(0, 0, 50, 50),
            stack_area: Rect::new(0, 40, 50, 15),
            bet_area: Rect::new(25, 50, 30, 15),
            avatar_area: Rect::new(0, 0, 25, 25),
            card_area: None,
        }];
        let changes = tracker.track(&rois, &frame).unwrap();
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].new_status, SeatStatus::Active);
        assert!(changes[0].prev_status.is_none());
    }

    #[test]
    fn test_track_no_change() {
        let mut tracker = SeatTracker::default();
        let frame = make_frame(640, 480, 180);
        let rois = vec![SeatRoi {
            seat_id: SeatId::new(0),
            seat_area: Rect::new(0, 0, 50, 50),
            stack_area: Rect::new(0, 40, 50, 15),
            bet_area: Rect::new(25, 50, 30, 15),
            avatar_area: Rect::new(0, 0, 25, 25),
            card_area: None,
        }];
        tracker.track(&rois, &frame).unwrap();
        let changes = tracker.track(&rois, &frame).unwrap();
        assert!(changes.is_empty());
    }
}
