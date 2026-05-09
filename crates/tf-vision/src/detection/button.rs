//! 行动按钮检测（Fold / Call / Raise / All-in）
//!
//! 注意：仅用于"判断当前是否轮到 hero 行动"，**不**用于推导动作语义。
//! 动作推导仍走 stack/pot diff。

use tf_core::{Frame, Rect, TfError};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActionButtonKind {
    Fold,
    Check,
    Call,
    Raise,
    AllIn,
}

#[derive(Debug, Clone)]
pub struct ActionButtonState {
    pub kind: ActionButtonKind,
    pub visible: bool,
    pub enabled: bool,
}

pub struct ActionButtonDetector;

impl ActionButtonDetector {
    /// 返回当前帧上各按钮的可见 / 可点击状态
    /// TODO(detail-impl): 模板匹配 + 颜色判定（disabled 通常变灰）
    pub fn detect(
        &self,
        _action_button_rois: &[Rect; 4],
        _frame: &Frame,
    ) -> Result<Vec<ActionButtonState>, TfError> {
        todo!("ActionButtonDetector::detect")
    }
}
