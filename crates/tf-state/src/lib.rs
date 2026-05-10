//! tf-state — 状态机 / 行动重建 / 回合引擎 / 校验
//!
//! 这是 TableFlow 的"单一状态真相来源"（Single Source of Truth）。
//! 所有视觉特征经 ActionReconstructor 推导成 `ReconstructedAction`，
//! 进入 `TableStateMachine::process_event` 后才正式更新 TableState。

pub mod history;
pub mod machine;
pub mod reconstructor;
pub mod round;
pub mod state;
pub mod tracker;
pub mod validator;

pub use history::*;
pub use machine::*;
pub use reconstructor::*;
pub use round::*;
pub use state::*;
pub use tracker::*;
pub use validator::*;
