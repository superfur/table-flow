//! Frame capture（DXGI Desktop Duplication / Windows Graphics Capture）

pub mod dxgi;
pub mod fps;
pub mod macos;
pub mod window;

pub use dxgi::*;
pub use fps::*;
pub use macos::*;
pub use window::*;

use async_trait::async_trait;

use tf_core::{Frame, Rect, TableId, TfError};

/// 捕获一帧的契约
#[async_trait]
pub trait FrameCapture: Send + Sync {
    async fn capture_frame(&mut self) -> Result<CapturedFrame, TfError>;

    /// 返回当前捕获区域；若窗口位置变化，应在内部更新 ROI
    fn current_region(&self) -> Rect;

    /// 强制重新探测窗口位置（recovery 路径调用）
    async fn rediscover_window(&mut self) -> Result<Rect, TfError>;
}

/// 一次捕获产出的帧
#[derive(Debug, Clone)]
pub struct CapturedFrame {
    pub timestamp_ms: i64,
    pub table_id: TableId,
    pub frame: Frame,
    pub frame_number: u64,
    /// 捕获本身耗费的时间（纯捕获 latency，不含后续处理）
    pub capture_latency_us: u64,
}
