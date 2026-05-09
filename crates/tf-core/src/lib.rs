//! tf-core — 全局类型 / 错误 / 事件 / 配置
//!
//! 这是 TableFlow 工作区的根 crate，所有其他 crate 都依赖它。
//! 不应包含任何业务逻辑，只放跨 crate 共享的类型与契约。

pub mod config;
pub mod error;
pub mod events;
pub mod frame;
pub mod types;

pub use config::*;
pub use error::*;
pub use events::*;
pub use frame::*;
pub use types::*;
