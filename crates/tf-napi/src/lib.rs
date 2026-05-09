//! tf-napi — Electron native addon (napi-rs)
//!
//! 架构骨架阶段：先把 bridge / commands / events / types 模块拆开。
//! detail-impl 阶段引入 napi/napi-derive 后填充 `#[napi]` 宏。

pub mod bridge;
pub mod commands;
pub mod events;
pub mod types;
