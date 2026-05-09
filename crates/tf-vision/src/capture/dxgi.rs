//! DXGI Desktop Duplication API 实现的 `FrameCapture`。

use async_trait::async_trait;

use tf_core::{Rect, TableId, TfError};

use super::{CapturedFrame, FrameCapture};

/// 基于 DXGI Desktop Duplication 的帧捕获器
pub struct DxgiCapture {
    pub table_id: TableId,
    pub window_title_pattern: String,
    pub region: Rect,
}

impl DxgiCapture {
    /// TODO(detail-impl):
    ///   - 找到 window handle
    ///   - 创建 D3D11 Device & DXGI OutputDuplication
    ///   - 缓存 staging texture
    pub fn new(_table_id: TableId, _window_title_pattern: String) -> Result<Self, TfError> {
        todo!("DxgiCapture::new")
    }
}

#[async_trait]
impl FrameCapture for DxgiCapture {
    /// TODO(detail-impl):
    ///   - AcquireNextFrame
    ///   - Map staging texture to CPU
    ///   - 拷贝到 Frame（按 region 裁剪 / BGRA→BGR）
    ///   - ReleaseFrame
    async fn capture_frame(&mut self) -> Result<CapturedFrame, TfError> {
        todo!("DxgiCapture::capture_frame")
    }

    fn current_region(&self) -> Rect {
        self.region
    }

    /// TODO(detail-impl): 重新枚举窗口、更新 region
    async fn rediscover_window(&mut self) -> Result<Rect, TfError> {
        todo!("DxgiCapture::rediscover_window")
    }
}
