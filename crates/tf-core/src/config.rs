//! 全局配置类型。
//!
//! 这些类型是序列化到磁盘 / 通过 IPC 传输的，必须保持 forward-compatible。

use serde::{Deserialize, Serialize};

use crate::frame::Rect;
use crate::types::SeatId;

// =============================================================================
// 线程 / 推理配置
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThreadConfig {
    pub tokio_worker_threads: usize,
    pub rayon_worker_threads: usize,
    pub onnx_intra_threads: usize,
    pub onnx_inter_threads: usize,
    pub max_tables: usize,
}

impl Default for ThreadConfig {
    fn default() -> Self {
        let cores = num_cpus::get().max(2);
        Self {
            tokio_worker_threads: cores,
            rayon_worker_threads: (cores * 3) / 4,
            onnx_intra_threads: 2,
            onnx_inter_threads: 2,
            max_tables: 8,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InferenceConfig {
    pub card_model_path: std::path::PathBuf,
    pub digit_model_path: std::path::PathBuf,
    pub onnx_intra_threads: usize,
    pub onnx_inter_threads: usize,
    pub card_session_count: usize,
    pub digit_session_count: usize,
    pub use_gpu: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CaptureBackend {
    Dxgi,
    WindowsGraphicsCapture,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManagerConfig {
    pub max_tables: usize,
    pub fps_per_table: u32,
    pub capture_backend: CaptureBackend,
    pub thread_config: ThreadConfig,
    pub inference_config: InferenceConfig,
}

// =============================================================================
// 盲注 / Ante
// =============================================================================

/// 一桌的盲注配置 + 当前轮的下注上下文。
/// 由 state machine 维护，但定义在 tf-core 以便所有 crate 共用。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BlindsInfo {
    pub small_blind: f64,
    pub big_blind: f64,
    pub ante: f64,
    pub straddle: f64,
    /// 当前轮中的最大下注额（用于计算 to_call）
    pub current_max_bet: f64,
    /// 当前 street 上一次合法加注的"加注幅度"（用于计算 min_raise）
    pub last_raise_size: f64,
}

// =============================================================================
// 校准 Profile（一份 = 客户端 × 主题 × 分辨率）
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalibrationProfile {
    pub profile_id: String,
    pub theme_id: String,
    pub client_signature: ClientSignature,
    pub calibration: TableCalibration,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientSignature {
    /// 用于自动识别窗口的正则
    pub window_title_regex: String,
    pub window_class: Option<String>,
    /// 桌面主色（HSV / BGR）的弱信号
    pub felt_color_hint: Option<(u8, u8, u8)>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableCalibration {
    pub resolution: (u32, u32),
    /// 所有 ROI 都用归一化坐标 (x, y, w, h) ∈ [0, 1]
    pub hole_card_positions: [NormalizedRect; 2],
    pub community_card_positions: [NormalizedRect; 5],
    pub pot_position: NormalizedRect,
    pub seat_positions: Vec<SeatCalibration>,
    pub dealer_button_region: NormalizedRect,
    pub action_button_regions: [NormalizedRect; 4],
    /// 用户手动指定的 Hero 座位（MVP 必填）
    pub hero_seat: Option<SeatId>,
    pub blinds: BlindsInfo,
    pub digit_ocr_regions: DigitOcrRegions,
    pub theme_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SeatCalibration {
    pub seat_id: SeatId,
    pub seat_region: NormalizedRect,
    pub stack_region: NormalizedRect,
    pub bet_region: NormalizedRect,
    pub avatar_region: NormalizedRect,
    pub card_region: Option<NormalizedRect>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DigitOcrRegions {
    pub pot: Option<NormalizedRect>,
    pub seat_stacks: Vec<Option<NormalizedRect>>,
    pub seat_bets: Vec<Option<NormalizedRect>>,
}

/// 归一化矩形：x/y/w/h ∈ [0.0, 1.0]，应用时乘以实际分辨率得到像素 `Rect`
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct NormalizedRect {
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
}

impl NormalizedRect {
    pub const fn new(x: f64, y: f64, w: f64, h: f64) -> Self {
        Self { x, y, w, h }
    }

    /// 映射到具体分辨率的像素 `Rect`
    pub fn to_pixel_rect(self, resolution: (u32, u32)) -> Rect {
        let (w, h) = (resolution.0 as f64, resolution.1 as f64);
        Rect::new(
            (self.x * w).round() as i32,
            (self.y * h).round() as i32,
            (self.w * w).round() as u32,
            (self.h * h).round() as u32,
        )
    }
}
