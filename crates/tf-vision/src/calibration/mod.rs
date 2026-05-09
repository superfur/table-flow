//! 桌面区域校准 / Profile 加载与匹配

pub mod auto;

pub use auto::*;

use std::path::Path;

use tf_core::{CalibrationProfile, TfError};

/// 加载磁盘上的所有 calibration profile（resources/profiles/*.json）
/// TODO(detail-impl)
pub fn load_profiles(_dir: &Path) -> Result<Vec<CalibrationProfile>, TfError> {
    todo!("calibration::load_profiles")
}

/// 给定一个 WindowInfo，从一组 profile 中选出最匹配的那个
/// 匹配规则：window_title_regex + felt_color_hint + (可选) class_name
/// TODO(detail-impl)
pub fn match_profile<'a>(
    _profiles: &'a [CalibrationProfile],
    _window_title: &str,
    _felt_color_hint: Option<(u8, u8, u8)>,
) -> Option<&'a CalibrationProfile> {
    todo!("calibration::match_profile")
}
