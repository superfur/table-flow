//! 推荐引擎输出

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecOutput {
    /// 推荐动作的字符串表示（例：`"raise_pot"`、`"call"`、`"fold"`）
    pub action: String,
    /// 推荐下注额（绝对额）
    pub amount: f64,
    /// 推荐自身的置信度（0–1）
    pub confidence: f64,
    /// 候选动作概率分布（与 SDK 输出一致）
    pub distribution: HashMap<String, f64>,
    /// 期望 EV（BB / bb 单位由调用方决定）
    pub ev: f64,
    /// SDK 端处理耗时（ms），用于诊断
    pub processing_time_ms: f64,
}
