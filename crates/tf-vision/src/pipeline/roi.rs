//! ROI 管理：把 TableCalibration 归一化坐标映射成本帧像素区域

use tf_core::{Frame, Rect, SeatId, TableCalibration, TfError};

#[derive(Debug, Clone)]
pub struct TableRoi {
    pub hole_cards: [Rect; 2],
    pub community_cards: [Rect; 5],
    pub pot_area: Rect,
    pub player_seats: Vec<SeatRoi>,
    pub dealer_button: Rect,
    pub action_buttons: [Rect; 4],
}

#[derive(Debug, Clone)]
pub struct SeatRoi {
    pub seat_id: SeatId,
    pub seat_area: Rect,
    pub stack_area: Rect,
    pub bet_area: Rect,
    pub avatar_area: Rect,
    pub card_area: Option<Rect>,
}

pub struct RoiManager {
    pub calibration: TableCalibration,
    cached_resolution: Option<(u32, u32)>,
    cached_roi: Option<TableRoi>,
}

impl RoiManager {
    pub fn new(calibration: TableCalibration) -> Self {
        Self {
            calibration,
            cached_resolution: None,
            cached_roi: None,
        }
    }

    pub fn update_calibration(&mut self, calibration: TableCalibration) {
        self.calibration = calibration;
        self.cached_resolution = None;
        self.cached_roi = None;
    }

    pub fn extract(&mut self, frame: &Frame) -> Result<TableRoi, TfError> {
        let res = (frame.width, frame.height);
        if self.cached_resolution == Some(res) {
            if let Some(ref roi) = self.cached_roi {
                return Ok(roi.clone());
            }
        }

        let cal = &self.calibration;
        let hole_cards = [
            cal.hole_card_positions[0].to_pixel_rect(res),
            cal.hole_card_positions[1].to_pixel_rect(res),
        ];
        let community_cards = [
            cal.community_card_positions[0].to_pixel_rect(res),
            cal.community_card_positions[1].to_pixel_rect(res),
            cal.community_card_positions[2].to_pixel_rect(res),
            cal.community_card_positions[3].to_pixel_rect(res),
            cal.community_card_positions[4].to_pixel_rect(res),
        ];
        let pot_area = cal.pot_position.to_pixel_rect(res);
        let dealer_button = cal.dealer_button_region.to_pixel_rect(res);
        let action_buttons = [
            cal.action_button_regions[0].to_pixel_rect(res),
            cal.action_button_regions[1].to_pixel_rect(res),
            cal.action_button_regions[2].to_pixel_rect(res),
            cal.action_button_regions[3].to_pixel_rect(res),
        ];

        let player_seats: Vec<SeatRoi> = cal
            .seat_positions
            .iter()
            .map(|s| SeatRoi {
                seat_id: s.seat_id,
                seat_area: s.seat_region.to_pixel_rect(res),
                stack_area: s.stack_region.to_pixel_rect(res),
                bet_area: s.bet_region.to_pixel_rect(res),
                avatar_area: s.avatar_region.to_pixel_rect(res),
                card_area: s.card_region.map(|r| r.to_pixel_rect(res)),
            })
            .collect();

        let roi = TableRoi {
            hole_cards,
            community_cards,
            pot_area,
            player_seats,
            dealer_button,
            action_buttons,
        };

        self.cached_resolution = Some(res);
        self.cached_roi = Some(roi.clone());
        Ok(roi)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tf_core::{BlindsInfo, DigitOcrRegions, NormalizedRect, SeatCalibration};

    fn make_calibration() -> TableCalibration {
        TableCalibration {
            resolution: (1920, 1080),
            hole_card_positions: [
                NormalizedRect::new(0.1, 0.5, 0.05, 0.08),
                NormalizedRect::new(0.17, 0.5, 0.05, 0.08),
            ],
            community_card_positions: [
                NormalizedRect::new(0.35, 0.4, 0.05, 0.08),
                NormalizedRect::new(0.42, 0.4, 0.05, 0.08),
                NormalizedRect::new(0.49, 0.4, 0.05, 0.08),
                NormalizedRect::new(0.56, 0.4, 0.05, 0.08),
                NormalizedRect::new(0.63, 0.4, 0.05, 0.08),
            ],
            pot_position: NormalizedRect::new(0.45, 0.3, 0.1, 0.04),
            seat_positions: vec![
                SeatCalibration {
                    seat_id: SeatId::new(0),
                    seat_region: NormalizedRect::new(0.0, 0.0, 0.1, 0.1),
                    stack_region: NormalizedRect::new(0.0, 0.08, 0.1, 0.03),
                    bet_region: NormalizedRect::new(0.05, 0.12, 0.05, 0.03),
                    avatar_region: NormalizedRect::new(0.0, 0.0, 0.05, 0.05),
                    card_region: Some(NormalizedRect::new(0.02, 0.0, 0.04, 0.06)),
                },
            ],
            dealer_button_region: NormalizedRect::new(0.3, 0.35, 0.03, 0.03),
            action_button_regions: [
                NormalizedRect::new(0.4, 0.85, 0.08, 0.04),
                NormalizedRect::new(0.5, 0.85, 0.08, 0.04),
                NormalizedRect::new(0.6, 0.85, 0.08, 0.04),
                NormalizedRect::new(0.7, 0.85, 0.08, 0.04),
            ],
            hero_seat: Some(SeatId::new(0)),
            blinds: BlindsInfo::default(),
            digit_ocr_regions: DigitOcrRegions::default(),
            theme_id: "test".to_string(),
        }
    }

    fn make_frame(w: u32, h: u32) -> Frame {
        Frame {
            width: w,
            height: h,
            stride: w * 4,
            format: tf_core::PixelFormat::Bgra8,
            data: std::sync::Arc::new(vec![0u8; (w * h * 4) as usize]),
        }
    }

    #[test]
    fn test_extract_roi() {
        let cal = make_calibration();
        let mut mgr = RoiManager::new(cal);
        let frame = make_frame(1920, 1080);
        let roi = mgr.extract(&frame).unwrap();

        assert_eq!(roi.player_seats.len(), 1);
        assert_eq!(roi.hole_cards.len(), 2);
        assert_eq!(roi.community_cards.len(), 5);
    }

    #[test]
    fn test_cached_roi() {
        let cal = make_calibration();
        let mut mgr = RoiManager::new(cal);
        let frame = make_frame(1920, 1080);
        let r1 = mgr.extract(&frame).unwrap();
        let r2 = mgr.extract(&frame).unwrap();
        assert_eq!(r1.hole_cards[0], r2.hole_cards[0]);
    }

    #[test]
    fn test_update_calibration_clears_cache() {
        let cal = make_calibration();
        let mut mgr = RoiManager::new(cal);
        let frame = make_frame(1920, 1080);
        mgr.extract(&frame).unwrap();
        mgr.update_calibration(make_calibration());
        assert!(mgr.cached_roi.is_none());
    }
}
