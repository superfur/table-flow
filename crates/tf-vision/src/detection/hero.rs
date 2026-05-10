//! Hero 座位识别（手动校准 + 正面手牌检测 fallback）

use std::time::{Duration, Instant};

use tf_core::{Frame, PixelFormat, Rect, SeatId, TfError};

use crate::detection::card::CardDetector;
use crate::pipeline::SeatRoi;

pub struct HeroDetector {
    pub manual_hero: Option<SeatId>,
    pub last_detected: Option<(SeatId, Instant)>,
    pub confidence_window: Duration,
}

impl Default for HeroDetector {
    fn default() -> Self {
        Self {
            manual_hero: None,
            last_detected: None,
            confidence_window: Duration::from_secs(10),
        }
    }
}

impl HeroDetector {
    pub fn with_manual(seat: SeatId) -> Self {
        Self {
            manual_hero: Some(seat),
            ..Default::default()
        }
    }

    pub async fn detect(
        &mut self,
        seat_rois: &[SeatRoi],
        frame: &Frame,
        card_detector: &dyn CardDetector,
    ) -> Result<Option<SeatId>, TfError> {
        if let Some(manual) = self.manual_hero {
            self.last_detected = Some((manual, Instant::now()));
            return Ok(Some(manual));
        }

        if let Some((seat, ts)) = self.last_detected {
            if ts.elapsed() < self.confidence_window {
                return Ok(Some(seat));
            }
        }

        let detected = self.detect_by_face_up_cards(seat_rois, frame, card_detector)?;

        if let Some(seat) = detected {
            self.last_detected = Some((seat, Instant::now()));
        }

        Ok(detected.or_else(|| self.last_detected.map(|(s, _)| s)))
    }

    fn detect_by_face_up_cards(
        &self,
        seat_rois: &[SeatRoi],
        frame: &Frame,
        card_detector: &dyn CardDetector,
    ) -> Result<Option<SeatId>, TfError> {
        for seat_roi in seat_rois {
            if let Some(card_rect) = &seat_roi.card_area {
                let card_img = crop_roi(frame, card_rect);
                if card_detector.is_face_up_card(&card_img) {
                    return Ok(Some(seat_roi.seat_id));
                }
            }
        }
        Ok(None)
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
    use async_trait::async_trait;
    use std::sync::Arc;
    use tf_core::{Card, TfError};

    struct MockCardDetector;

    #[async_trait]
    impl CardDetector for MockCardDetector {
        async fn detect(
            &self,
            _hole_rois: &[Rect; 2],
            _community_rois: &[Rect; 5],
            _frame: &Frame,
        ) -> Result<crate::features::CardDetectionResult, TfError> {
            Ok(crate::features::CardDetectionResult::default())
        }

        async fn detect_single(
            &self,
            _roi: &Rect,
            _frame: &Frame,
        ) -> Result<Option<Card>, TfError> {
            Ok(None)
        }

        fn is_face_up_card(&self, roi: &Frame) -> bool {
            let pixel_count = roi.width as usize * roi.height as usize;
            if pixel_count == 0 || roi.data.is_empty() {
                return false;
            }
            let lum = roi.data[0] as f64 * 0.299
                + roi.data.get(1).copied().unwrap_or(roi.data[0]) as f64 * 0.587
                + roi.data.get(2).copied().unwrap_or(roi.data[0]) as f64 * 0.114;
            lum > 120.0
        }
    }

    fn make_frame(w: u32, h: u32, fill: u8) -> Frame {
        Frame {
            width: w,
            height: h,
            stride: w * 3,
            format: PixelFormat::Rgb8,
            data: Arc::new(vec![fill; (w * h * 3) as usize]),
        }
    }

    fn make_seat_roi(id: u8, has_card: bool, fill: u8) -> (SeatRoi, u8) {
        let roi = SeatRoi {
            seat_id: SeatId::new(id),
            seat_area: Rect::new(0, 0, 80, 80),
            stack_area: Rect::new(0, 60, 80, 20),
            bet_area: Rect::new(40, 80, 40, 15),
            avatar_area: Rect::new(0, 0, 40, 40),
            card_area: if has_card {
                Some(Rect::new(10, 10, 40, 60))
            } else {
                None
            },
        };
        (roi, fill)
    }

    #[tokio::test]
    async fn test_manual_hero_takes_priority() {
        let mut detector = HeroDetector::with_manual(SeatId::new(3));
        let card_det = MockCardDetector;
        let frame = make_frame(640, 480, 30);
        let result = detector.detect(&[], &frame, &card_det).await.unwrap();
        assert_eq!(result, Some(SeatId::new(3)));
    }

    #[tokio::test]
    async fn test_auto_detect_face_up_cards() {
        let mut detector = HeroDetector::default();
        let card_det = MockCardDetector;
        let frame = make_frame(640, 480, 200);
        let (seat0, _) = make_seat_roi(0, true, 200);
        let (seat1, _) = make_seat_roi(1, true, 30);
        let rois = vec![seat0, seat1];
        let result = detector.detect(&rois, &frame, &card_det).await.unwrap();
        assert_eq!(result, Some(SeatId::new(0)));
    }

    #[tokio::test]
    async fn test_no_hero_when_all_dark() {
        let mut detector = HeroDetector::default();
        let card_det = MockCardDetector;
        let frame = make_frame(640, 480, 30);
        let (seat0, _) = make_seat_roi(0, true, 30);
        let rois = vec![seat0];
        let result = detector.detect(&rois, &frame, &card_det).await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_no_card_area_skips_seat() {
        let mut detector = HeroDetector::default();
        let card_det = MockCardDetector;
        let frame = make_frame(640, 480, 200);
        let (seat0, _) = make_seat_roi(0, false, 200);
        let rois = vec![seat0];
        let result = detector.detect(&rois, &frame, &card_det).await.unwrap();
        assert!(result.is_none());
    }
}
