//! Table discovery —— 自动发现可识别的扑克客户端窗口

use serde::{Deserialize, Serialize};

use tf_core::TfError;
use tf_vision::WindowInfo;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveredTable {
    pub table_id: String,
    pub window: WindowInfo,
    /// 匹配到的 Calibration profile id；None 表示需要用户手动校准
    pub matched_profile_id: Option<String>,
}

pub struct TableDiscovery;

impl TableDiscovery {
    /// 周期性扫描所有窗口 → 与 known calibration profiles 匹配 →
    /// 返回当前可识别为"扑克牌桌"的窗口列表。
    /// TODO(detail-impl):
    ///   - 调用 tf_vision::enumerate_windows
    ///   - 用 tf_vision::match_profile 匹配
    ///   - 生成 stable table_id（建议 hash window handle + title）
    pub async fn scan() -> Result<Vec<DiscoveredTable>, TfError> {
        todo!("TableDiscovery::scan")
    }
}
