//! 自动校准（v1.1+ 能力，MVP 阶段允许返回 NotImplemented）

use tf_core::{Frame, TableCalibration, TfError};

pub struct AutoCalibrator {
    pub theme_id: String,
}

impl AutoCalibrator {
    pub fn new(theme_id: String) -> Self {
        Self { theme_id }
    }

    /// 从一帧抽取 TableCalibration（自动模式）
    /// MVP 直接返回 `Err(TfError::Calibration(...))`
    /// TODO(detail-impl, v1.1):
    ///   - Hough Circle / 绿色 felt mask 检测桌面椭圆
    ///   - findContours 定位卡牌矩形
    ///   - 色彩分割定位筹码区域
    ///   - 模板匹配定位 dealer button
    pub fn calibrate_from_frame(&self, _frame: &Frame) -> Result<TableCalibration, TfError> {
        Err(TfError::Calibration(
            "AutoCalibrator not available in MVP — use manual calibration".into(),
        ))
    }
}
