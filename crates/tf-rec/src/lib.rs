//! tf-rec — Recommendation Engine integration
//!
//! MVP 方案：通过 stdio JSON-RPC 与 Node.js sidecar 通信，
//! 复用已有的 TypeScript SDK（不重写）。

pub mod cache;
pub mod engine;
pub mod input;
pub mod output;
pub mod sidecar;

pub use cache::*;
pub use engine::*;
pub use input::*;
pub use output::*;
pub use sidecar::*;
