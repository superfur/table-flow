//! tf-table — 多桌编排
//!
//! `TableManager` 持有所有 `TableHandle`，每个 handle 内部驱动一条
//! capture → vision → state → rec 的完整 pipeline，并向上发出事件。

pub mod discovery;
pub mod handle;
pub mod manager;

pub use discovery::*;
pub use handle::*;
pub use manager::*;
