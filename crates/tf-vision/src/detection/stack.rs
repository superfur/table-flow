//! 筹码追踪

use std::collections::{HashMap, VecDeque};
use std::sync::Arc;

use tf_core::{Frame, Rect, SeatId, TfError};
use tf_inference::Ocr;

use crate::features::StackChange;
use crate::pipeline::SeatRoi;

#[derive(Debug, Clone)]
pub struct StackBaseline {
    pub seat_id: SeatId,
    pub known_value: Option<f64>,
    pub pixel_count: f64,
    pub calibration_factor: f64,
}

#[derive(Debug, Clone)]
pub struct StackSnapshot {
    pub seat_id: SeatId,
    pub estimated_value: f64,
    pub confidence: f32,
    pub timestamp_ms: i64,
}

pub struct StackTracker {
    pub baselines: HashMap<SeatId, StackBaseline>,
    pub history: HashMap<SeatId, VecDeque<StackSnapshot>>,
    pub history_window: usize,
}

impl Default for StackTracker {
    fn default() -> Self {
        Self {
            baselines: HashMap::new(),
            history: HashMap::new(),
            history_window: 30,
        }
    }
}

impl StackTracker {
    pub async fn track(
        &mut self,
        seat_rois: &[SeatRoi],
        _frame: &Frame,
        ocr: Option<&Arc<dyn Ocr>>,
    ) -> Result<Vec<StackChange>, TfError> {
        let mut changes = Vec::new();
        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as i64;

        for seat_roi in seat_rois {
            let estimated = if let Some(ocr_engine) = ocr {
                let roi_frame = crop_roi(_frame, &seat_roi.stack_area);
                match ocr_engine.recognize_digits(&roi_frame).await {
                    Ok(Some(v)) => (v, 0.97f32),
                    _ => (0.0, 0.0),
                }
            } else {
                (0.0, 0.0)
            };

            if estimated.0 == 0.0 {
                continue;
            }

            let snapshot = StackSnapshot {
                seat_id: seat_roi.seat_id,
                estimated_value: estimated.0,
                confidence: estimated.1,
                timestamp_ms: now_ms,
            };

            if let Some(history) = self.history.get(&seat_roi.seat_id) {
                if let Some(prev) = history.back() {
                    let delta = snapshot.estimated_value - prev.estimated_value;
                    if delta.abs() > 0.5 {
                        changes.push(StackChange {
                            seat_id: seat_roi.seat_id,
                            prev_estimated: prev.estimated_value,
                            curr_estimated: snapshot.estimated_value,
                            delta,
                            confidence: snapshot.confidence.min(prev.confidence),
                        });
                    }
                }
            }

            let history = self
                .history
                .entry(seat_roi.seat_id)
                .or_insert_with(|| VecDeque::with_capacity(self.history_window));
            history.push_back(snapshot);
            if history.len() > self.history_window {
                history.pop_front();
            }
        }

        Ok(changes)
    }

    pub fn calibrate_from_known_value(
        &mut self,
        seat_id: SeatId,
        known_stack: f64,
        frame: &Frame,
        stack_roi: Rect,
    ) -> Result<(), TfError> {
        let roi_frame = crop_roi(frame, &stack_roi);
        let pixel_count = count_bright_pixels(&roi_frame);
        let calibration_factor = if known_stack > 0.0 {
            pixel_count / known_stack
        } else {
            100.0
        };

        self.baselines.insert(
            seat_id,
            StackBaseline {
                seat_id,
                known_value: Some(known_stack),
                pixel_count,
                calibration_factor,
            },
        );
        Ok(())
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
        tf_core::PixelFormat::Gray8 => 1,
        tf_core::PixelFormat::Bgr8 | tf_core::PixelFormat::Rgb8 => 3,
        tf_core::PixelFormat::Bgra8 => 4,
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
        data: Arc::new(data),
    }
}

fn count_bright_pixels(frame: &Frame) -> f64 {
    let channels = match frame.format {
        tf_core::PixelFormat::Gray8 => 1,
        tf_core::PixelFormat::Bgr8 | tf_core::PixelFormat::Rgb8 => 3,
        tf_core::PixelFormat::Bgra8 => 4,
    };
    let pixel_count = frame.width as usize * frame.height as usize;
    let mut count = 0usize;
    for i in 0..pixel_count {
        let off = i * channels;
        if off < frame.data.len() {
            let brightness: f64 = frame.data[off] as f64;
            if brightness > 50.0 {
                count += 1;
            }
        }
    }
    count as f64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default() {
        let tracker = StackTracker::default();
        assert!(tracker.baselines.is_empty());
        assert!(tracker.history.is_empty());
    }

    #[test]
    fn test_calibrate() {
        let mut tracker = StackTracker::default();
        let frame = Frame {
            width: 100, height: 50, stride: 300,
            format: tf_core::PixelFormat::Rgb8,
            data: Arc::new(vec![128u8; 15000]),
        };
        let result = tracker.calibrate_from_known_value(
            SeatId::new(0), 100.0, &frame, Rect::new(0, 0, 50, 25),
        );
        assert!(result.is_ok());
        assert!(tracker.baselines.contains_key(&SeatId::new(0)));
        let baseline = &tracker.baselines[&SeatId::new(0)];
        assert_eq!(baseline.known_value, Some(100.0));
    }

    #[tokio::test]
    async fn test_track_no_ocr() {
        let mut tracker = StackTracker::default();
        let frame = Frame {
            width: 640, height: 480, stride: 1920,
            format: tf_core::PixelFormat::Rgb8,
            data: Arc::new(vec![128u8; 640 * 480 * 3]),
        };
        let rois = vec![SeatRoi {
            seat_id: SeatId::new(0),
            seat_area: Rect::new(0, 0, 100, 100),
            stack_area: Rect::new(0, 0, 80, 30),
            bet_area: Rect::new(50, 50, 60, 20),
            avatar_area: Rect::new(0, 0, 40, 40),
            card_area: None,
        }];
        let changes = tracker.track(&rois, &frame, None).await.unwrap();
        assert!(changes.is_empty());
    }
}
