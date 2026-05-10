//! Card classifier trait（抽象层，便于测试 mock）。

use std::sync::Arc;

use async_trait::async_trait;

use tf_core::{Card, Frame, Rank, Suit, TfError};

use crate::prepost;
use crate::session::{CardClassificationInput, CardClassificationOutput};

const CARD_MODEL_WIDTH: u32 = 64;
const CARD_MODEL_HEIGHT: u32 = 90;

/// 卡牌识别结果（含 suit / rank 推断与原始 logits）
#[derive(Debug, Clone)]
pub struct CardPrediction {
    pub card: Card,
    pub raw: CardClassificationOutput,
}

/// 抽象的卡牌分类器。
#[async_trait]
pub trait CardClassifier: Send + Sync {
    async fn classify(&self, roi: &Frame) -> Result<Option<CardPrediction>, TfError>;
}

/// 标准 52-class 到 (Suit, Rank) 的映射。
///
/// 约定 index：suit_idx * 13 + rank_idx
///   suit_idx: 0=Spades 1=Hearts 2=Diamonds 3=Clubs
///   rank_idx: 0=Two ... 12=Ace
pub fn class_id_to_card(class_id: u32) -> Option<(Suit, Rank)> {
    if class_id >= 52 {
        return None;
    }
    let suit = match class_id / 13 {
        0 => Suit::Spades,
        1 => Suit::Hearts,
        2 => Suit::Diamonds,
        3 => Suit::Clubs,
        _ => return None,
    };
    let rank = match class_id % 13 {
        0 => Rank::Two,
        1 => Rank::Three,
        2 => Rank::Four,
        3 => Rank::Five,
        4 => Rank::Six,
        5 => Rank::Seven,
        6 => Rank::Eight,
        7 => Rank::Nine,
        8 => Rank::Ten,
        9 => Rank::Jack,
        10 => Rank::Queen,
        11 => Rank::King,
        12 => Rank::Ace,
        _ => return None,
    };
    Some((suit, rank))
}

/// 把任意尺寸的 Frame ROI 标准化为 64×90 RGB tensor。
pub fn frame_to_card_input(roi: &Frame) -> Result<CardClassificationInput, TfError> {
    let rgb = match roi.format {
        tf_core::PixelFormat::Rgb8 => roi.clone(),
        tf_core::PixelFormat::Bgra8 => prepost::bgra_to_rgb(roi)?,
        tf_core::PixelFormat::Bgr8 => {
            // BGR → RGB: swap channels
            let pixel_count = roi.width as usize * roi.height as usize;
            let src = &roi.data;
            let mut dst = Vec::with_capacity(pixel_count * 3);
            for i in 0..pixel_count {
                let off = i * 3;
                dst.push(src[off + 2]);
                dst.push(src[off + 1]);
                dst.push(src[off]);
            }
            Frame {
                width: roi.width,
                height: roi.height,
                stride: roi.width * 3,
                format: tf_core::PixelFormat::Rgb8,
                data: Arc::new(dst),
            }
        }
        tf_core::PixelFormat::Gray8 => {
            return Err(TfError::Vision(
                "cannot convert Gray8 to RGB for card classification".into(),
            ));
        }
    };

    let resized = prepost::resize(&rgb, CARD_MODEL_WIDTH, CARD_MODEL_HEIGHT)?;

    Ok(CardClassificationInput {
        data: resized.data.clone(),
        width: CARD_MODEL_WIDTH,
        height: CARD_MODEL_HEIGHT,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_class_id_to_card_all_suits() {
        let (s, r) = class_id_to_card(0).unwrap();
        assert_eq!(s, Suit::Spades);
        assert_eq!(r, Rank::Two);

        let (s, r) = class_id_to_card(12).unwrap();
        assert_eq!(s, Suit::Spades);
        assert_eq!(r, Rank::Ace);

        let (s, r) = class_id_to_card(13).unwrap();
        assert_eq!(s, Suit::Hearts);
        assert_eq!(r, Rank::Two);

        let (s, r) = class_id_to_card(26).unwrap();
        assert_eq!(s, Suit::Diamonds);
        assert_eq!(r, Rank::Two);

        let (s, r) = class_id_to_card(39).unwrap();
        assert_eq!(s, Suit::Clubs);
        assert_eq!(r, Rank::Two);

        let (s, r) = class_id_to_card(51).unwrap();
        assert_eq!(s, Suit::Clubs);
        assert_eq!(r, Rank::Ace);
    }

    #[test]
    fn test_class_id_out_of_range() {
        assert!(class_id_to_card(52).is_none());
        assert!(class_id_to_card(100).is_none());
    }

    #[test]
    fn test_frame_to_card_input_rgb() {
        use tf_core::PixelFormat;
        let data = vec![128u8; 200 * 300 * 3];
        let frame = Frame {
            width: 200,
            height: 300,
            stride: 600,
            format: PixelFormat::Rgb8,
            data: Arc::new(data),
        };
        let input = frame_to_card_input(&frame).unwrap();
        assert_eq!(input.width, 64);
        assert_eq!(input.height, 90);
        assert_eq!(input.data.len(), 64 * 90 * 3);
    }

    #[test]
    fn test_frame_to_card_input_bgra() {
        use tf_core::PixelFormat;
        let data = vec![128u8; 200 * 300 * 4];
        let frame = Frame {
            width: 200,
            height: 300,
            stride: 800,
            format: PixelFormat::Bgra8,
            data: Arc::new(data),
        };
        let input = frame_to_card_input(&frame).unwrap();
        assert_eq!(input.width, 64);
        assert_eq!(input.height, 90);
    }

    #[test]
    fn test_class_id_52_cards_unique() {
        use std::collections::HashSet;
        let cards: HashSet<(Suit, Rank)> = (0..52)
            .filter_map(class_id_to_card)
            .collect();
        assert_eq!(cards.len(), 52);
    }
}
