//! Window enumeration / tracking（Win32 API 封装）

use serde::{Deserialize, Serialize};

use tf_core::{Rect, TfError};

/// 简化版的窗口元数据（不直接暴露 HWND，避免 Send 限制）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowInfo {
    /// 不透明 ID（detail-impl 中可装 `HWND as isize`）
    pub handle: u64,
    pub title: String,
    pub class_name: Option<String>,
    pub bounds: Rect,
}

/// 枚举所有窗口，按 title regex 过滤。
/// TODO(detail-impl): EnumWindows + GetWindowText + GetWindowRect
pub fn enumerate_windows(_title_regex: &str) -> Result<Vec<WindowInfo>, TfError> {
    todo!("enumerate_windows")
}

/// 给定窗口 handle，返回当前 bounds（用于追踪窗口移动）
/// TODO(detail-impl): GetWindowRect by HWND
pub fn get_window_bounds(_handle: u64) -> Result<Rect, TfError> {
    todo!("get_window_bounds")
}
