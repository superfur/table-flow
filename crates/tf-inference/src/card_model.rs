//! Card classifier trait（抽象层，便于测试 mock）。

use async_trait::async_trait;

use tf_core::{Card, Frame, Rank, Suit, TfError};

use crate::session::{CardClassificationInput, CardClassificationOutput};

/// 卡牌识别结果（含 suit / rank 推断与原始 logits）
#[derive(Debug, Clone)]
pub struct CardPrediction {
    pub card: Card,
    /// 用于调试的原始网络输出
    pub raw: CardClassificationOutput,
}

/// 抽象的卡牌分类器。
///
/// detail-impl 阶段：实现一个 `OnnxCardClassifier`，内部持有 `Arc<InferencePool>`。
/// 测试时可注入 `MockCardClassifier` 直接返回固定 Card。
#[async_trait]
pub trait CardClassifier: Send + Sync {
    /// 给一张卡牌 ROI 图（已经被 detection 模块裁剪到接近 64×90），
    /// 返回识别结果。如果无法识别（confidence 不达标）返回 `None`。
    async fn classify(&self, roi: &Frame) -> Result<Option<CardPrediction>, TfError>;
}

// =============================================================================
// 工具：class_id → (Suit, Rank) 映射（52 张牌的标准 indexing）
// =============================================================================

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

// =============================================================================
// 输入预处理（架构骨架：仅声明，detail-impl 实现）
// =============================================================================

/// 把任意尺寸的 Frame ROI 标准化为 64×90 RGB tensor。
/// TODO(detail-impl): resize + normalize + channel reorder
pub fn frame_to_card_input(_roi: &Frame) -> Result<CardClassificationInput, TfError> {
    todo!("frame_to_card_input — resize ROI to 64x90 RGB tensor")
}
