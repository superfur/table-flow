//! 卡牌检测（手牌 + 公共牌）

use async_trait::async_trait;

use tf_core::{Card, Frame, PixelFormat, Rect, TfError};

use crate::features::CardDetectionResult;

#[async_trait]
pub trait CardDetector: Send + Sync {
    async fn detect(
        &self,
        hole_rois: &[Rect; 2],
        community_rois: &[Rect; 5],
        frame: &Frame,
    ) -> Result<CardDetectionResult, TfError>;

    async fn detect_single(&self, roi: &Rect, frame: &Frame) -> Result<Option<Card>, TfError>;

    fn is_face_up_card(&self, roi: &Frame) -> bool;
}

pub struct DefaultCardDetector;

#[async_trait]
impl CardDetector for DefaultCardDetector {
    async fn detect(
        &self,
        hole_rois: &[Rect; 2],
        community_rois: &[Rect; 5],
        frame: &Frame,
    ) -> Result<CardDetectionResult, TfError> {
        let mut hole_cards_arr = [None, None];
        for (i, roi) in hole_rois.iter().enumerate() {
            hole_cards_arr[i] = self.detect_single(roi, frame).await.ok().flatten();
        }
        let hole_cards = match (hole_cards_arr[0], hole_cards_arr[1]) {
            (Some(a), Some(b)) => Some([a, b]),
            _ => None,
        };

        let mut community_cards = Vec::new();
        for roi in community_rois {
            if let Some(card) = self.detect_single(roi, frame).await.ok().flatten() {
                community_cards.push(card);
            }
        }

        Ok(CardDetectionResult {
            hole_cards,
            community_cards,
        })
    }

    async fn detect_single(&self, _roi: &Rect, _frame: &Frame) -> Result<Option<Card>, TfError> {
        // Mock: no detection without real ONNX model
        Ok(None)
    }

    fn is_face_up_card(&self, roi: &Frame) -> bool {
        // Heuristic: face-up cards have higher average brightness than card backs
        if roi.data.is_empty() {
            return false;
        }
        let channels = match roi.format {
            PixelFormat::Gray8 => 1,
            PixelFormat::Bgr8 | PixelFormat::Rgb8 => 3,
            PixelFormat::Bgra8 => 4,
        };
        let pixel_count = roi.width as usize * roi.height as usize;
        if pixel_count == 0 {
            return false;
        }
        let sample_step = (pixel_count / 100).max(1);
        let mut brightness_sum: f64 = 0.0;
        let mut sample_count: f64 = 0.0;
        for i in (0..pixel_count).step_by(sample_step) {
            let off = i * channels;
            if off + channels <= roi.data.len() {
            let b = roi.data[off];
            let g = roi.data.get(off + 1).copied().unwrap_or(b);
            let r = roi.data.get(off + 2).copied().unwrap_or(b);
            let lum = 0.114 * b as f64 + 0.587 * g as f64 + 0.299 * r as f64;
            brightness_sum += lum;
                sample_count += 1.0;
            }
        }
        if sample_count == 0.0 {
            return false;
        }
        let avg_brightness = brightness_sum / sample_count;
        avg_brightness > 120.0
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
    fn test_face_up_bright() {
        let det = DefaultCardDetector;
        let frame = make_frame(64, 90, 200);
        assert!(det.is_face_up_card(&frame));
    }

    #[test]
    fn test_face_down_dark() {
        let det = DefaultCardDetector;
        let frame = make_frame(64, 90, 30);
        assert!(!det.is_face_up_card(&frame));
    }

    #[test]
    fn test_face_up_empty() {
        let det = DefaultCardDetector;
        let frame = Frame {
            width: 0, height: 0, stride: 0,
            format: PixelFormat::Rgb8,
            data: Arc::new(vec![]),
        };
        assert!(!det.is_face_up_card(&frame));
    }

    #[tokio::test]
    async fn test_detect_returns_empty() {
        let det = DefaultCardDetector;
        let frame = make_frame(640, 480, 128);
        let rois_hole = [Rect::new(0, 0, 64, 90), Rect::new(70, 0, 64, 90)];
        let rois_comm = [
            Rect::new(200, 200, 64, 90),
            Rect::new(270, 200, 64, 90),
            Rect::new(340, 200, 64, 90),
            Rect::new(410, 200, 64, 90),
            Rect::new(480, 200, 64, 90),
        ];
        let result = det.detect(&rois_hole, &rois_comm, &frame).await.unwrap();
        assert!(result.hole_cards.is_none());
        assert!(result.community_cards.is_empty());
    }
}
