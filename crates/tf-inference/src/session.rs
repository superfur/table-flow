//! ONNX Session Pool。
//!
//! 两种模式：
//!   - `mock` feature（默认）：mock session，不需要 ONNX Runtime
//!   - `real-onnx` feature：`ort::session::Session`，需要 ONNX Runtime 库
//!
//! 关键设计：
//!   1. `InferencePool` 持有 card/digit 两个 session 池
//!   2. `crossbeam::queue::ArrayQueue` 做 round-robin 借还
//!   3. `tokio::sync::Semaphore` 控制并发数
//!   4. 推理在 `spawn_blocking` 中执行

use std::sync::Arc;

use tokio::sync::Semaphore;
use tf_core::{InferenceConfig, TfError};

use crate::card_model::class_id_to_card;

// ---------------------------------------------------------------------------
// Session 类型抽象
// ---------------------------------------------------------------------------

#[cfg(not(feature = "real-onnx"))]
pub struct OpaqueSession(());

#[cfg(not(feature = "real-onnx"))]
impl OpaqueSession {
    pub fn placeholder() -> Self {
        Self(())
    }
}

#[cfg(feature = "real-onnx")]
use parking_lot::Mutex;

#[cfg(feature = "real-onnx")]
pub struct OrtSession(Mutex<ort::session::Session>);

#[cfg(feature = "real-onnx")]
impl OrtSession {
    fn new(path: &std::path::Path, threads: usize) -> Result<Self, TfError> {
        let session = ort::session::Session::builder()
            .and_then(|b| b.with_intra_threads(threads))
            .and_then(|b| b.with_inter_threads(1))
            .and_then(|b| b.commit_from_file(path))
            .map_err(|e| {
                TfError::Inference(format!(
                    "Failed to load ONNX model {:?}: {}",
                    path, e
                ))
            })?;
        Ok(Self(Mutex::new(session)))
    }
}

#[cfg(not(feature = "real-onnx"))]
type Sess = OpaqueSession;
#[cfg(feature = "real-onnx")]
type Sess = OrtSession;

// ---------------------------------------------------------------------------
// InferencePool
// ---------------------------------------------------------------------------

pub struct InferencePool {
    inner: Arc<PoolInner>,
    card_semaphore: Arc<Semaphore>,
    digit_semaphore: Arc<Semaphore>,
}

struct PoolInner {
    config: InferenceConfig,
    card_queue: crossbeam::queue::ArrayQueue<Arc<Sess>>,
    digit_queue: crossbeam::queue::ArrayQueue<Arc<Sess>>,
}

impl InferencePool {
    pub fn new(config: InferenceConfig) -> Result<Self, TfError> {
        let n_card = config.card_session_count.max(1);
        let n_digit = config.digit_session_count.max(1);

        let card_sessions: Vec<Arc<Sess>> = (0..n_card)
            .map(|_| new_card_session(&config))
            .collect::<Result<Vec<_>, _>>()?;

        let digit_sessions: Vec<Arc<Sess>> = (0..n_digit)
            .map(|_| new_digit_session(&config))
            .collect::<Result<Vec<_>, _>>()?;

        let card_queue = crossbeam::queue::ArrayQueue::new(n_card);
        for s in &card_sessions {
            let _ = card_queue.push(s.clone());
        }

        let digit_queue = crossbeam::queue::ArrayQueue::new(n_digit);
        for s in &digit_sessions {
            let _ = digit_queue.push(s.clone());
        }

        Ok(Self {
            inner: Arc::new(PoolInner {
                config,
                card_queue,
                digit_queue,
            }),
            card_semaphore: Arc::new(Semaphore::new(n_card)),
            digit_semaphore: Arc::new(Semaphore::new(n_digit)),
        })
    }

    pub async fn classify_card(
        &self,
        input: CardClassificationInput,
    ) -> Result<CardClassificationOutput, TfError> {
        let _permit = self.card_semaphore.acquire().await.map_err(|e| {
            TfError::Inference(e.to_string())
        })?;

        let session = self.inner.card_queue.pop();
        let inner = self.inner.clone();

        let (result, session) = tokio::task::spawn_blocking(move || {
            let r = run_card_inference(session.as_deref(), &input);
            (r, session)
        })
        .await
        .map_err(|e| TfError::Inference(e.to_string()))?;

        if let Some(s) = session {
            let _ = inner.card_queue.push(s);
        }

        result
    }

    pub async fn recognize_digits(
        &self,
        input: DigitInput,
    ) -> Result<DigitOutput, TfError> {
        let _permit = self.digit_semaphore.acquire().await.map_err(|e| {
            TfError::Inference(e.to_string())
        })?;

        let session = self.inner.digit_queue.pop();
        let inner = self.inner.clone();

        let (result, session) = tokio::task::spawn_blocking(move || {
            let r = run_digit_inference(session.as_deref(), &input);
            (r, session)
        })
        .await
        .map_err(|e| TfError::Inference(e.to_string()))?;

        if let Some(s) = session {
            let _ = inner.digit_queue.push(s);
        }

        result
    }

    pub async fn shutdown(self) -> Result<(), TfError> {
        drop(self.inner);
        Ok(())
    }

    pub fn config(&self) -> InferenceConfig {
        self.inner.config.clone()
    }
}

// ---------------------------------------------------------------------------
// Session 创建
// ---------------------------------------------------------------------------

#[cfg(not(feature = "real-onnx"))]
fn new_card_session(_config: &InferenceConfig) -> Result<Arc<Sess>, TfError> {
    Ok(Arc::new(OpaqueSession::placeholder()))
}

#[cfg(not(feature = "real-onnx"))]
fn new_digit_session(_config: &InferenceConfig) -> Result<Arc<Sess>, TfError> {
    Ok(Arc::new(OpaqueSession::placeholder()))
}

#[cfg(feature = "real-onnx")]
fn new_card_session(config: &InferenceConfig) -> Result<Arc<Sess>, TfError> {
    if !config.card_model_path.exists() {
        return Err(TfError::Inference(format!(
            "Card model not found: {:?}",
            config.card_model_path
        )));
    }
    Ok(Arc::new(OrtSession::new(
        &config.card_model_path,
        config.onnx_intra_threads,
    )?))
}

#[cfg(feature = "real-onnx")]
fn new_digit_session(config: &InferenceConfig) -> Result<Arc<Sess>, TfError> {
    if !config.digit_model_path.exists() {
        return Err(TfError::Inference(format!(
            "Digit model not found: {:?}",
            config.digit_model_path
        )));
    }
    Ok(Arc::new(OrtSession::new(
        &config.digit_model_path,
        config.onnx_intra_threads,
    )?))
}

// ---------------------------------------------------------------------------
// 推理实现
// ---------------------------------------------------------------------------

#[cfg(not(feature = "real-onnx"))]
fn run_card_inference(
    _session: Option<&Sess>,
    _input: &CardClassificationInput,
) -> Result<CardClassificationOutput, TfError> {
    Ok(CardClassificationOutput {
        class_id: 0,
        confidence: 0.5,
        suit_logits: [0.25; 4],
    })
}

#[cfg(feature = "real-onnx")]
fn run_card_inference(
    session: Option<&Sess>,
    input: &CardClassificationInput,
) -> Result<CardClassificationOutput, TfError> {
    let session = session
        .ok_or_else(|| TfError::Inference("No card session available".into()))?;
    let mut guard = session.0.lock();
    let sess = &mut *guard;

    let input_tensor = ndarray::Array3::from_shape_vec(
        (1, input.height as usize, input.width as usize * 3),
        input.data.to_vec(),
    )
    .map_err(|e| TfError::Inference(format!("Tensor shape error: {}", e)))?;

    let ort_input = ort::value::Value::from_array(input_tensor.into_dyn())
        .map_err(|e| TfError::Inference(format!("Value creation failed: {}", e)))?;

    let input_name = sess.inputs[0].name.to_string();
    let outputs = sess
        .run(ort::inputs![&input_name => ort_input])
        .map_err(|e| TfError::Inference(format!("Card inference failed: {}", e)))?;

    let output_view = outputs[0]
        .try_extract_array::<f32>()
        .map_err(|e| TfError::Inference(format!("Card output extract failed: {}", e)))?;

    let flat = output_view
        .as_slice()
        .ok_or_else(|| TfError::Inference("Card output not contiguous".into()))?;

    let (class_id, confidence) = flat
        .iter()
        .enumerate()
        .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
        .map(|(i, &c)| (i as u32, c))
        .unwrap_or((0, 0.0));

    Ok(CardClassificationOutput {
        class_id,
        confidence,
        suit_logits: [0.25; 4],
    })
}

#[cfg(not(feature = "real-onnx"))]
fn run_digit_inference(
    _session: Option<&Sess>,
    _input: &DigitInput,
) -> Result<DigitOutput, TfError> {
    Ok(DigitOutput {
        digits: String::new(),
        avg_confidence: 0.0,
    })
}

#[cfg(feature = "real-onnx")]
fn run_digit_inference(
    session: Option<&Sess>,
    input: &DigitInput,
) -> Result<DigitOutput, TfError> {
    let session = session
        .ok_or_else(|| TfError::Inference("No digit session available".into()))?;
    let mut guard = session.0.lock();
    let sess = &mut *guard;

    let normalized: Vec<f32> = input.data.iter().map(|&b| b as f32 / 255.0).collect();

    let input_tensor = ndarray::Array3::from_shape_vec(
        (1, input.height as usize, input.width as usize),
        normalized,
    )
    .map_err(|e| TfError::Inference(format!("Digit tensor shape error: {}", e)))?;

    let ort_input = ort::value::Value::from_array(input_tensor.into_dyn())
        .map_err(|e| TfError::Inference(format!("Digit value creation failed: {}", e)))?;

    let input_name = sess.inputs[0].name.to_string();
    let outputs = sess
        .run(ort::inputs![&input_name => ort_input])
        .map_err(|e| TfError::Inference(format!("Digit inference failed: {}", e)))?;

    let output_view = outputs[0]
        .try_extract_array::<f32>()
        .map_err(|e| TfError::Inference(format!("Digit output extract failed: {}", e)))?;

    let flat = output_view
        .as_slice()
        .ok_or_else(|| TfError::Inference("Digit output not contiguous".into()))?;

    let charset = "0123456789.$,KMBkmb ";
    let num_classes = charset.chars().count().max(1);

    let mut decoded = String::new();
    let mut total_conf = 0.0_f32;
    let mut count = 0usize;

    for row in flat.chunks(num_classes) {
        if let Some((idx, &conf)) = row.iter().enumerate().max_by(|(_, a), (_, b)| {
            a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal)
        }) {
            if let Some(ch) = charset.chars().nth(idx) {
                if ch != ' ' {
                    decoded.push(ch);
                }
            }
            total_conf += conf;
            count += 1;
        }
    }

    Ok(DigitOutput {
        digits: decoded,
        avg_confidence: if count > 0 { total_conf / count as f32 } else { 0.0 },
    })
}

// ---------------------------------------------------------------------------
// 输入 / 输出类型
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct CardClassificationInput {
    pub data: Arc<Vec<u8>>,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone)]
pub struct CardClassificationOutput {
    pub class_id: u32,
    pub confidence: f32,
    pub suit_logits: [f32; 4],
}

impl CardClassificationOutput {
    pub fn to_card(&self) -> Option<tf_core::Card> {
        let (suit, rank) = class_id_to_card(self.class_id)?;
        Some(tf_core::Card {
            suit,
            rank,
            confidence: self.confidence,
        })
    }
}

#[derive(Debug, Clone)]
pub struct DigitInput {
    pub data: Arc<Vec<u8>>,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone)]
pub struct DigitOutput {
    pub digits: String,
    pub avg_confidence: f32,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use tf_core::InferenceConfig;

    fn make_config() -> InferenceConfig {
        InferenceConfig {
            card_model_path: PathBuf::from("card.onnx"),
            digit_model_path: PathBuf::from("digit.onnx"),
            onnx_intra_threads: 1,
            onnx_inter_threads: 1,
            card_session_count: 2,
            digit_session_count: 2,
            use_gpu: false,
        }
    }

    #[test]
    fn test_pool_new() {
        let pool = InferencePool::new(make_config()).unwrap();
        assert_eq!(pool.config().card_session_count, 2);
    }

    #[tokio::test]
    async fn test_classify_card_mock() {
        let pool = InferencePool::new(make_config()).unwrap();
        let input = CardClassificationInput {
            data: Arc::new(vec![128u8; 64 * 90 * 3]),
            width: 64,
            height: 90,
        };
        let output = pool.classify_card(input).await.unwrap();
        assert_eq!(output.class_id, 0);
        assert_eq!(output.confidence, 0.5);
    }

    #[tokio::test]
    async fn test_recognize_digits_mock() {
        let pool = InferencePool::new(make_config()).unwrap();
        let input = DigitInput {
            data: Arc::new(vec![128u8; 32 * 100]),
            width: 100,
            height: 32,
        };
        let output = pool.recognize_digits(input).await.unwrap();
        assert!(output.digits.is_empty());
    }

    #[tokio::test]
    async fn test_shutdown() {
        let pool = InferencePool::new(make_config()).unwrap();
        pool.shutdown().await.unwrap();
    }

    #[test]
    fn test_output_to_card() {
        let output = CardClassificationOutput {
            class_id: 12,
            confidence: 0.99,
            suit_logits: [1.0, 0.0, 0.0, 0.0],
        };
        let card = output.to_card().unwrap();
        assert_eq!(card.suit, tf_core::Suit::Spades);
        assert_eq!(card.rank, tf_core::Rank::Ace);
    }

    #[test]
    fn test_output_to_card_invalid() {
        let output = CardClassificationOutput {
            class_id: 99,
            confidence: 0.1,
            suit_logits: [0.25; 4],
        };
        assert!(output.to_card().is_none());
    }
}
