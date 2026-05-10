//! Pot 追踪

use std::collections::VecDeque;
use std::sync::Arc;

use crc32fast::Hasher;
use tf_core::{Frame, Rect, TfError};
use tf_inference::Ocr;

use crate::features::PotChange;

pub struct PotTracker {
    pub last_value: f64,
    pub last_pixel_hash: u64,
    pub change_history: VecDeque<PotChange>,
}

impl Default for PotTracker {
    fn default() -> Self {
        Self {
            last_value: 0.0,
            last_pixel_hash: 0,
            change_history: VecDeque::with_capacity(64),
        }
    }
}

impl PotTracker {
    pub fn compute_hash(&self, pot_img: &Frame) -> u64 {
        let mut hasher = Hasher::new();
        hasher.update(&pot_img.data);
        hasher.finalize() as u64
    }

    pub async fn track(
        &mut self,
        pot_roi: &Rect,
        frame: &Frame,
        ocr: Option<&Arc<dyn Ocr>>,
    ) -> Result<Option<PotChange>, TfError> {
        let pot_img = crop_roi(frame, pot_roi);
        let current_hash = self.compute_hash(&pot_img);

        if current_hash == self.last_pixel_hash {
            return Ok(None);
        }
        self.last_pixel_hash = current_hash;

        let current_value = if let Some(ocr_engine) = ocr {
            match ocr_engine.recognize_digits(&pot_img).await {
                Ok(Some(v)) => v,
                _ => return Ok(None),
            }
        } else {
            return Ok(None);
        };

        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as i64;

        let change = PotChange {
            prev_value: self.last_value,
            new_value: current_value,
            delta: current_value - self.last_value,
            timestamp_ms: now_ms,
        };

        self.last_value = current_value;
        self.change_history.push_back(change.clone());

        Ok(Some(change))
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compute_hash() {
        let tracker = PotTracker::default();
        let frame = Frame {
            width: 10, height: 10, stride: 30,
            format: tf_core::PixelFormat::Rgb8,
            data: Arc::new(vec![42u8; 300]),
        };
        let h1 = tracker.compute_hash(&frame);
        let h2 = tracker.compute_hash(&frame);
        assert_eq!(h1, h2);

        let frame2 = Frame {
            width: 10, height: 10, stride: 30,
            format: tf_core::PixelFormat::Rgb8,
            data: Arc::new(vec![99u8; 300]),
        };
        let h3 = tracker.compute_hash(&frame2);
        assert_ne!(h1, h3);
    }

    #[tokio::test]
    async fn test_track_no_ocr() {
        let mut tracker = PotTracker::default();
        let frame = Frame {
            width: 640, height: 480, stride: 1920,
            format: tf_core::PixelFormat::Rgb8,
            data: Arc::new(vec![128u8; 640 * 480 * 3]),
        };
        let result = tracker.track(&Rect::new(300, 200, 100, 30), &frame, None).await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_track_same_hash_skips() {
        let mut tracker = PotTracker::default();
        let frame = Frame {
            width: 640, height: 480, stride: 1920,
            format: tf_core::PixelFormat::Rgb8,
            data: Arc::new(vec![128u8; 640 * 480 * 3]),
        };
        let roi = Rect::new(300, 200, 100, 30);
        tracker.track(&roi, &frame, None).await.unwrap();
        let result = tracker.track(&roi, &frame, None).await.unwrap();
        assert!(result.is_none());
    }
}
