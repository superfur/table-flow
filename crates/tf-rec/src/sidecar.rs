//! Node.js sidecar 子进程 + JSON-RPC over stdio。
//!
//! MVP 集成方案：
//! ```text
//!   tf-rec (Rust)  ──spawn──▶  node ./rec-sidecar/index.js
//!                  ◀──stdin/stdout (JSON-RPC 2.0, line-delimited)
//! ```
//!
//! 协议：
//!   - `rec.recommend(input)`     → `RecOutput`
//!   - `rec.health()`             → `{ ok: true, version: "..." }`
//!   - `rec.shutdown()`           → `{ ok: true }`
//! 默认超时 500ms，连续 3 次失败自动重启 sidecar。

use std::path::PathBuf;
use std::sync::atomic::AtomicU64;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use tokio::sync::oneshot;

use tf_core::TfError;

use crate::engine::RecEngine;
use crate::input::RecInput;
use crate::output::RecOutput;

pub const DEFAULT_TIMEOUT: Duration = Duration::from_millis(500);
pub const MAX_CONSECUTIVE_FAILURES: u32 = 3;

#[derive(Debug, Clone)]
pub struct SidecarConfig {
    pub node_executable: PathBuf,
    pub script_path: PathBuf,
    pub request_timeout: Duration,
    pub max_consecutive_failures: u32,
}

impl Default for SidecarConfig {
    fn default() -> Self {
        Self {
            node_executable: PathBuf::from("node"),
            script_path: PathBuf::from("./rec-sidecar/index.js"),
            request_timeout: DEFAULT_TIMEOUT,
            max_consecutive_failures: MAX_CONSECUTIVE_FAILURES,
        }
    }
}

// =============================================================================
// JSON-RPC 协议类型
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcRequest<T> {
    pub jsonrpc: &'static str,
    pub id: u64,
    pub method: String,
    pub params: T,
}

#[derive(Debug, Clone, Deserialize)]
pub struct JsonRpcResponse {
    pub jsonrpc: String,
    pub id: u64,
    #[serde(default)]
    pub result: Option<serde_json::Value>,
    #[serde(default)]
    pub error: Option<JsonRpcError>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct JsonRpcError {
    pub code: i32,
    pub message: String,
}

// =============================================================================
// RecSidecar
// =============================================================================

pub struct RecSidecar {
    config: SidecarConfig,
    next_id: AtomicU64,
    pending: Arc<Mutex<std::collections::HashMap<u64, oneshot::Sender<JsonRpcResponse>>>>,
    /// detail-impl 阶段填入：tokio::process::Child / ChildStdin / 任务句柄
    inner: Mutex<Option<SidecarInner>>,
}

struct SidecarInner {
    // detail-impl: child handle / writer / reader_task_handle
}

impl RecSidecar {
    /// 启动 sidecar 子进程。
    /// TODO(detail-impl):
    ///   - tokio::process::Command::new(node).arg(script).spawn()
    ///   - 拿 stdin/stdout，启动一个 reader task：按行读 → 解析 JsonRpcResponse → 唤醒 pending
    ///   - 设置 panic-on-drop / 自动重启逻辑
    pub async fn spawn(_config: SidecarConfig) -> Result<Arc<Self>, TfError> {
        todo!("RecSidecar::spawn")
    }

    /// 发起一个 JSON-RPC 调用（带超时）。
    /// TODO(detail-impl)
    pub async fn call<P: Serialize, R: for<'de> Deserialize<'de>>(
        &self,
        _method: &str,
        _params: P,
    ) -> Result<R, TfError> {
        todo!("RecSidecar::call")
    }

    /// 重启子进程（连续失败时调用）
    pub async fn restart(&self) -> Result<(), TfError> {
        todo!("RecSidecar::restart")
    }

    pub async fn shutdown(&self) -> Result<(), TfError> {
        todo!("RecSidecar::shutdown")
    }

    pub fn config(&self) -> &SidecarConfig {
        &self.config
    }
}

#[async_trait]
impl RecEngine for RecSidecar {
    async fn recommend(&self, _input: RecInput) -> Result<RecOutput, TfError> {
        todo!("RecSidecar::recommend — call('rec.recommend', input)")
    }

    async fn health(&self) -> Result<(), TfError> {
        todo!("RecSidecar::health — call('rec.health', ())")
    }
}
