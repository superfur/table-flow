//! tf-integration-tests — 端到端集成测试和性能基准
//!
//! 验证 Vision → State → Rec 完整链路，
//! 以及多桌并发、性能基准等。

#[cfg(test)]
pub mod benches;
#[cfg(test)]
pub mod e2e;
#[cfg(test)]
pub mod fixtures;
#[cfg(test)]
pub mod multi_table;
#[cfg(test)]
pub mod sidecar;
