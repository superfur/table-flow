//! 全局共享的基础类型。
//!
//! 这些类型出现在多个 crate 的公开 API 中，必须保持稳定。
//! 业务逻辑（如何识别一张牌、如何推导一个动作）不在这里。

use serde::{Deserialize, Serialize};

// =============================================================================
// 标识符
// =============================================================================

/// 一张牌桌的唯一标识符（一般用 window handle hex / UUID 字符串）
pub type TableId = String;

/// 0..=9 的座位索引
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize,
)]
pub struct SeatId(pub u8);

impl SeatId {
    pub const fn new(idx: u8) -> Self {
        Self(idx)
    }
}

// =============================================================================
// 牌
// =============================================================================

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize,
)]
pub enum Suit {
    Spades,
    Hearts,
    Diamonds,
    Clubs,
}

impl Suit {
    pub const fn short_str(self) -> &'static str {
        match self {
            Suit::Spades => "s",
            Suit::Hearts => "h",
            Suit::Diamonds => "d",
            Suit::Clubs => "c",
        }
    }
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize,
)]
pub enum Rank {
    Two,
    Three,
    Four,
    Five,
    Six,
    Seven,
    Eight,
    Nine,
    Ten,
    Jack,
    Queen,
    King,
    Ace,
}

impl Rank {
    pub const fn value(self) -> u8 {
        match self {
            Rank::Two => 2,
            Rank::Three => 3,
            Rank::Four => 4,
            Rank::Five => 5,
            Rank::Six => 6,
            Rank::Seven => 7,
            Rank::Eight => 8,
            Rank::Nine => 9,
            Rank::Ten => 10,
            Rank::Jack => 11,
            Rank::Queen => 12,
            Rank::King => 13,
            Rank::Ace => 14,
        }
    }

    pub const fn short_str(self) -> &'static str {
        match self {
            Rank::Two => "2",
            Rank::Three => "3",
            Rank::Four => "4",
            Rank::Five => "5",
            Rank::Six => "6",
            Rank::Seven => "7",
            Rank::Eight => "8",
            Rank::Nine => "9",
            Rank::Ten => "T",
            Rank::Jack => "J",
            Rank::Queen => "Q",
            Rank::King => "K",
            Rank::Ace => "A",
        }
    }
}

/// 一张被识别出的扑克牌。
///
/// `confidence` 是视觉识别的置信度（0.0–1.0）。
/// **注意**：因为 `confidence` 是 `f32`，`Card` 不实现 `Hash / Eq`。
/// 需要按花色+点数判定唯一性时请用 `(card.suit, card.rank)`。
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Card {
    pub suit: Suit,
    pub rank: Rank,
    pub confidence: f32,
}

// =============================================================================
// 回合 / 阶段
// =============================================================================

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize,
)]
pub enum Street {
    Preflop,
    Flop,
    Turn,
    River,
    Showdown,
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize,
)]
pub enum TablePhase {
    Waiting,
    Playing,
    Showdown,
    Cleanup,
}

// =============================================================================
// 动作 / 盲注
// =============================================================================

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ActionType {
    Fold,
    Check,
    Call,
    /// 当前 street 没人下注，自己开第一注。amount 是绝对下注额。
    Bet(f64),
    /// 加注。f64 是 raise-to 的绝对额（不是 raise-by）。
    Raise(f64),
    /// All-in。f64 是本次 push 进底池的金额。
    AllIn(f64),
    /// 强制盲注 / Ante / Straddle，**不进入 GTO 推荐的 action_history**。
    PostBlind(BlindKind),
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize,
)]
pub enum BlindKind {
    SmallBlind,
    BigBlind,
    Straddle,
    Ante,
}

// =============================================================================
// 座位状态
// =============================================================================

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize,
)]
pub enum SeatStatus {
    Empty,
    SittingOut,
    Active,
    Folded,
    AllIn,
}

// =============================================================================
// 动作来源（用于调试 / 置信度计算）
// =============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ActionSource {
    StackDiff,
    PotDiff,
    SeatStatusChange,
    Combined,
}
