//! Node.js sidecar 子进程 + JSON-RPC over stdio。
//!
//! ```text
//!   tf-rec (Rust)  ──spawn──▶  node ./rec-sidecar/index.js
//!                  ◀──stdin/stdout (JSON-RPC 2.0, line-delimited)
//! ```

use std::io::{BufRead, Write};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, AtomicU32, Ordering};
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

pub const DEFAULT_TIMEOUT: Duration = Duration::from_millis(2000);
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

struct SidecarInner {
    stdin: std::process::ChildStdin,
    _child: std::process::Child,
}

pub struct RecSidecar {
    config: SidecarConfig,
    next_id: AtomicU64,
    pending: Arc<Mutex<std::collections::HashMap<u64, oneshot::Sender<JsonRpcResponse>>>>,
    consecutive_failures: AtomicU32,
    inner: Mutex<Option<SidecarInner>>,
}

impl RecSidecar {
    pub async fn spawn(config: SidecarConfig) -> Result<Arc<Self>, TfError> {
        let (inner, stdout) = Self::start_process(&config)?;

        let pending: Arc<Mutex<std::collections::HashMap<u64, oneshot::Sender<JsonRpcResponse>>>> =
            Arc::new(Mutex::new(std::collections::HashMap::new()));
        let pending_reader = pending.clone();

        std::thread::spawn(move || {
            let reader = std::io::BufReader::new(stdout);
            for line in reader.lines() {
                match line {
                    Ok(line_str) => {
                        if let Ok(resp) = serde_json::from_str::<JsonRpcResponse>(&line_str) {
                            let mut p = pending_reader.lock();
                            if let Some(tx) = p.remove(&resp.id) {
                                let _ = tx.send(resp);
                            }
                        }
                    }
                    Err(_) => break,
                }
            }
        });

        Ok(Arc::new(Self {
            config,
            next_id: AtomicU64::new(1),
            pending,
            consecutive_failures: AtomicU32::new(0),
            inner: Mutex::new(Some(inner)),
        }))
    }

    fn start_process(
        config: &SidecarConfig,
    ) -> Result<(SidecarInner, std::process::ChildStdout), TfError> {
        let mut child = std::process::Command::new(&config.node_executable)
            .arg(&config.script_path)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::null())
            .spawn()
            .map_err(|e| TfError::Recommendation(format!("Failed to spawn sidecar: {}", e)))?;

        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| TfError::Recommendation("Failed to get sidecar stdin".into()))?;

        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| TfError::Recommendation("Failed to get sidecar stdout".into()))?;

        Ok((SidecarInner { stdin, _child: child }, stdout))
    }

    pub async fn call<P: Serialize, R: for<'de> Deserialize<'de>>(
        &self,
        method: &str,
        params: P,
    ) -> Result<R, TfError> {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let request = JsonRpcRequest {
            jsonrpc: "2.0",
            id,
            method: method.to_string(),
            params,
        };

        let line = serde_json::to_string(&request)
            .map_err(|e| TfError::Recommendation(format!("JSON-RPC serialize error: {}", e)))?;

        let (tx, rx) = oneshot::channel();
        {
            let mut pending = self.pending.lock();
            pending.insert(id, tx);
        }

        {
            let mut guard = self.inner.lock();
            let inner = guard
                .as_mut()
                .ok_or_else(|| TfError::Recommendation("Sidecar not running".into()))?;

            writeln!(inner.stdin, "{}", line)
                .map_err(|e| TfError::Recommendation(format!("Sidecar write error: {}", e)))?;
            inner
                .stdin
                .flush()
                .map_err(|e| TfError::Recommendation(format!("Sidecar flush error: {}", e)))?;
        }

        let timeout = self.config.request_timeout;
        let result = tokio::time::timeout(timeout, rx).await;

        {
            let mut pending = self.pending.lock();
            pending.remove(&id);
        }

        match result {
            Ok(Ok(response)) => {
                self.consecutive_failures.store(0, Ordering::Relaxed);
                if let Some(error) = response.error {
                    return Err(TfError::Recommendation(format!(
                        "JSON-RPC error {}: {}",
                        error.code, error.message
                    )));
                }
                let value = response.result.ok_or_else(|| {
                    TfError::Recommendation("JSON-RPC response missing result".into())
                })?;
                serde_json::from_value(value).map_err(|e| {
                    TfError::Recommendation(format!("JSON-RPC deserialize error: {}", e))
                })
            }
            _ => {
                let failures = self.consecutive_failures.fetch_add(1, Ordering::Relaxed) + 1;
                if failures >= self.config.max_consecutive_failures {
                    let _ = self.restart().await;
                }
                Err(TfError::Recommendation(
                    "Sidecar request timeout or channel closed".into(),
                ))
            }
        }
    }

    pub async fn restart(&self) -> Result<(), TfError> {
        {
            let mut guard = self.inner.lock();
            if let Some(mut inner) = guard.take() {
                let _ = inner._child.kill();
                let _ = inner._child.wait();
            }
        }

        let (new_inner, stdout) = Self::start_process(&self.config)?;

        let pending_reader = self.pending.clone();
        std::thread::spawn(move || {
            let reader = std::io::BufReader::new(stdout);
            for line in reader.lines() {
                match line {
                    Ok(line_str) => {
                        if let Ok(resp) = serde_json::from_str::<JsonRpcResponse>(&line_str) {
                            let mut p = pending_reader.lock();
                            if let Some(tx) = p.remove(&resp.id) {
                                let _ = tx.send(resp);
                            }
                        }
                    }
                    Err(_) => break,
                }
            }
        });

        {
            let mut guard = self.inner.lock();
            *guard = Some(new_inner);
        }
        self.consecutive_failures.store(0, Ordering::Relaxed);

        Ok(())
    }

    pub async fn shutdown(&self) -> Result<(), TfError> {
        let mut guard = self.inner.lock();
        if let Some(mut inner) = guard.take() {
            let _ = writeln!(inner.stdin, r#"{{"jsonrpc":"2.0","id":0,"method":"rec.shutdown","params":{{}}}}"#);
            let _ = inner.stdin.flush();
            let _ = inner._child.kill();
            let _ = inner._child.wait();
        }
        Ok(())
    }

    pub fn config(&self) -> &SidecarConfig {
        &self.config
    }
}

#[async_trait]
impl RecEngine for RecSidecar {
    async fn recommend(&self, input: RecInput) -> Result<RecOutput, TfError> {
        self.call("rec.recommend", input).await
    }

    async fn health(&self) -> Result<(), TfError> {
        let _result: serde_json::Value = self.call("rec.health", ()).await?;
        Ok(())
    }
}

/// MockRecEngine — 测试用，返回固定结果。
pub struct MockRecEngine {
    pub output: RecOutput,
    pub healthy: bool,
}

impl MockRecEngine {
    pub fn new(output: RecOutput) -> Self {
        Self {
            output,
            healthy: true,
        }
    }
}

#[async_trait]
impl RecEngine for MockRecEngine {
    async fn recommend(&self, _input: RecInput) -> Result<RecOutput, TfError> {
        Ok(self.output.clone())
    }

    async fn health(&self) -> Result<(), TfError> {
        if self.healthy {
            Ok(())
        } else {
            Err(TfError::Recommendation("Mock engine unhealthy".into()))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sidecar_config_default() {
        let config = SidecarConfig::default();
        assert_eq!(config.node_executable, PathBuf::from("node"));
        assert_eq!(config.request_timeout, DEFAULT_TIMEOUT);
        assert_eq!(config.max_consecutive_failures, MAX_CONSECUTIVE_FAILURES);
    }

    #[test]
    fn test_json_rpc_request_serialize() {
        let req = JsonRpcRequest {
            jsonrpc: "2.0",
            id: 42,
            method: "rec.health".to_string(),
            params: (),
        };
        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("\"jsonrpc\":\"2.0\""));
        assert!(json.contains("\"id\":42"));
        assert!(json.contains("\"method\":\"rec.health\""));
    }

    #[test]
    fn test_json_rpc_response_deserialize() {
        let json = r#"{"jsonrpc":"2.0","id":1,"result":{"ok":true}}"#;
        let resp: JsonRpcResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.id, 1);
        assert!(resp.result.is_some());
        assert!(resp.error.is_none());
    }

    #[test]
    fn test_json_rpc_response_error() {
        let json = r#"{"jsonrpc":"2.0","id":2,"error":{"code":-32600,"message":"Invalid Request"}}"#;
        let resp: JsonRpcResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.id, 2);
        assert!(resp.result.is_none());
        let err = resp.error.unwrap();
        assert_eq!(err.code, -32600);
        assert_eq!(err.message, "Invalid Request");
    }

    #[tokio::test]
    async fn test_mock_rec_engine() {
        use std::collections::HashMap;
        use tf_core::{Card, Rank, Suit};

        let output = RecOutput {
            action: "fold".to_string(),
            amount: 0.0,
            confidence: 0.8,
            distribution: {
                let mut m = HashMap::new();
                m.insert("fold".to_string(), 0.8);
                m.insert("call".to_string(), 0.2);
                m
            },
            ev: -0.5,
            processing_time_ms: 10.0,
        };
        let engine = MockRecEngine::new(output.clone());

        let input = RecInput {
            hole_cards: [
                Card { suit: Suit::Spades, rank: Rank::Ace, confidence: 1.0 },
                Card { suit: Suit::Hearts, rank: Rank::King, confidence: 1.0 },
            ],
            community_cards: vec![],
            pot: 100.0,
            to_call: 50.0,
            min_raise: 100.0,
            stack: 500.0,
            street: tf_core::Street::Preflop,
            num_opponents: 3,
            action_history: vec![],
        };

        let result = engine.recommend(input).await.unwrap();
        assert_eq!(result.action, "fold");
        assert_eq!(result.confidence, 0.8);

        engine.health().await.unwrap();
    }

    #[tokio::test]
    async fn test_mock_rec_engine_unhealthy() {
        let output = RecOutput {
            action: "fold".to_string(),
            amount: 0.0,
            confidence: 0.8,
            distribution: Default::default(),
            ev: 0.0,
            processing_time_ms: 0.0,
        };
        let mut engine = MockRecEngine::new(output);
        engine.healthy = false;
        let result = engine.health().await;
        assert!(result.is_err());
    }

    #[test]
    fn test_spawn_fails_missing_script() {
        let config = SidecarConfig {
            script_path: PathBuf::from("/nonexistent/rec-sidecar/index.js"),
            ..Default::default()
        };
        assert!(config.script_path.to_str().unwrap().contains("nonexistent"));
    }

    #[tokio::test]
    async fn test_sidecar_e2e_health() {
        let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".to_string());
        let script = std::path::PathBuf::from(&manifest_dir)
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join("rec-sidecar/index.js");

        if !script.exists() {
            eprintln!("Skipping sidecar e2e test: script not found at {:?}", script);
            return;
        }

        let config = SidecarConfig {
            script_path: script,
            request_timeout: Duration::from_secs(5),
            ..Default::default()
        };

        let sidecar = RecSidecar::spawn(config).await.unwrap();

        let result: serde_json::Value = sidecar.call("rec.health", ()).await.unwrap();
        assert_eq!(result["ok"], true);
        assert_eq!(result["version"], "0.1.0");

        sidecar.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn test_sidecar_e2e_recommend() {
        let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".to_string());
        let script = std::path::PathBuf::from(&manifest_dir)
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join("rec-sidecar/index.js");

        if !script.exists() {
            eprintln!("Skipping sidecar e2e test: script not found at {:?}", script);
            return;
        }

        let config = SidecarConfig {
            script_path: script,
            request_timeout: Duration::from_secs(5),
            ..Default::default()
        };

        let sidecar = RecSidecar::spawn(config).await.unwrap();

        use tf_core::{Card, Rank, Suit};
        let input = RecInput {
            hole_cards: [
                Card { suit: Suit::Spades, rank: Rank::Ace, confidence: 0.99 },
                Card { suit: Suit::Spades, rank: Rank::Ace, confidence: 0.98 },
            ],
            community_cards: vec![],
            pot: 100.0,
            to_call: 50.0,
            min_raise: 100.0,
            stack: 500.0,
            street: tf_core::Street::Preflop,
            num_opponents: 2,
            action_history: vec![],
        };

        let output: RecOutput = sidecar.call("rec.recommend", input).await.unwrap();
        assert_eq!(output.action, "raise");
        assert!(output.amount > 0.0);
        assert!(output.confidence > 0.5);
        assert!(output.distribution.contains_key("raise"));
        assert!(output.processing_time_ms >= 0.0);

        sidecar.shutdown().await.unwrap();
    }
}
