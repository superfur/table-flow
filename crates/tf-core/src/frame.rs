//! Frame / Rect 类型占位
//!
//! 架构骨架阶段不引入 opencv 依赖（避免 system OpenCV 安装才能 cargo check）。
//! 我们定义不透明的 `Frame` 与 `Rect` 类型，detail-impl 阶段可：
//!   - 让 `Frame` 内部持有 `opencv::core::Mat`
//!   - 或换用 `image::DynamicImage` / 自定义内存布局
//! 公开 API 保持稳定。

/// 像素整型矩形（屏幕坐标 / ROI）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

impl Rect {
    pub const fn new(x: i32, y: i32, width: u32, height: u32) -> Self {
        Self { x, y, width, height }
    }
}

/// 一帧图像数据的不透明持有者
///
/// 占位实现：仅持有原始字节缓冲与元数据。
/// detail-impl 可以把它替换为 `Mat` 包装。
#[derive(Debug, Clone)]
pub struct Frame {
    pub width: u32,
    pub height: u32,
    pub stride: u32,
    pub format: PixelFormat,
    pub data: std::sync::Arc<Vec<u8>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PixelFormat {
    Bgra8,
    Bgr8,
    Rgb8,
    Gray8,
}
