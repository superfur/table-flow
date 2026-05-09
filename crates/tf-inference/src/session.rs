//! ONNX Session Pool（架构骨架阶段不引入 `ort`，只暴露契约）。
//!
//! detail-impl 阶段：
//!   - 用 `ort::Session` 替换 `OpaqueSession`
//!   - `new_card_session / new_digit_session` 内部 commit_from_file
//!   - `pool` 用 `crossbeam::queue::ArrayQueue` 做无锁 round-robin

use std::sync::Arc;

use parking_lot::Mutex;
use tf_core::{InferenceConfig, TfError};

/// 不透明的 session 句柄。detail-impl 替换为 `ort::Session`。
pub struct OpaqueSession {
    /// 占位字段，避免 zero-sized 引发的 unsafe Send 问题
    _marker: (),
}

impl OpaqueSession {
    pub fn placeholder() -> Self {
        Self { _marker: () }
    }
}

/// Session pool。
///
/// 注意：架构骨架阶段所有方法都是 `todo!()`。
/// detail-impl 阶段需要保证：
///   1. `Session` 不可 Clone，必须独立 commit_from_file 创建 N 个；
///   2. permit 必须覆盖整个推理过程（不能像旧文档那样提前 drop）；
///   3. 用 round-robin queue 而非固定 index 0。
pub struct InferencePool {
    inner: Arc<Mutex<InferencePoolInner>>,
}

struct InferencePoolInner {
    pub card_sessions: Vec<Arc<OpaqueSession>>,
    pub digit_sessions: Vec<Arc<OpaqueSession>>,
    pub config: InferenceConfig,
}

impl InferencePool {
    /// TODO(detail-impl):
    ///   - 加载 card / digit ONNX 模型
    ///   - 配置 GPU/CPU EP 优先级（CUDA > DirectML > CPU）
    ///   - 创建 N 个独立 Session（不要 clone）
    ///   - 初始化 round-robin queue + semaphore
    pub fn new(_config: InferenceConfig) -> Result<Self, TfError> {
        todo!("InferencePool::new — load ONNX sessions")
    }

    /// 借出一个 card session（异步、信号量保护）
    /// TODO(detail-impl): 借走 → spawn_blocking 推理 → 归还
    pub async fn classify_card(
        &self,
        _input: CardClassificationInput,
    ) -> Result<CardClassificationOutput, TfError> {
        todo!("InferencePool::classify_card")
    }

    /// 借出一个 digit session
    pub async fn recognize_digits(
        &self,
        _input: DigitInput,
    ) -> Result<DigitOutput, TfError> {
        todo!("InferencePool::recognize_digits")
    }

    /// 关闭所有 session（在 shutdown 时调用）
    pub async fn shutdown(self) -> Result<(), TfError> {
        todo!("InferencePool::shutdown")
    }

    pub fn config(&self) -> InferenceConfig {
        self.inner.lock().config.clone()
    }
}

// =============================================================================
// 输入 / 输出张量数据
// =============================================================================

/// 一个待推理的卡牌图像（已 crop 到 64×90 / RGB）
#[derive(Debug, Clone)]
pub struct CardClassificationInput {
    /// 行优先 RGB 像素，长度应为 64 * 90 * 3
    pub data: Arc<Vec<u8>>,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone)]
pub struct CardClassificationOutput {
    /// 52-class softmax 后的最大类
    pub class_id: u32,
    pub confidence: f32,
    /// 4-class suit 概率（spades / hearts / diamonds / clubs）
    pub suit_logits: [f32; 4],
}

/// 一个待识别的数字图像（已二值化 + 标准化到 32×100 灰度）
#[derive(Debug, Clone)]
pub struct DigitInput {
    /// 行优先灰度像素，长度应为 32 * 100
    pub data: Arc<Vec<u8>>,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone)]
pub struct DigitOutput {
    /// 识别出的数字序列（已 CTC 解码）
    pub digits: String,
    pub avg_confidence: f32,
}
