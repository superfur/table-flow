//! 卡牌检测（手牌 + 公共牌）

use async_trait::async_trait;

use tf_core::{Card, Frame, Rect, TfError};

use crate::features::CardDetectionResult;

/// 卡牌检测器抽象
#[async_trait]
pub trait CardDetector: Send + Sync {
    /// 检测一帧上的所有卡牌（手牌 + 公共牌）
    async fn detect(
        &self,
        hole_rois: &[Rect; 2],
        community_rois: &[Rect; 5],
        frame: &Frame,
    ) -> Result<CardDetectionResult, TfError>;

    /// 单独检测一张牌；某些 detector 在判定 Hero 时需要这个能力
    async fn detect_single(&self, roi: &Rect, frame: &Frame) -> Result<Option<Card>, TfError>;

    /// 给定一个 ROI 图像，判断是否是"正面牌"（即可识别的牌，而不是牌背）。
    /// HeroDetector 用这个判断哪张是 hero。
    fn is_face_up_card(&self, roi: &Frame) -> bool;
}

/// 默认实现：模板匹配 + ONNX 分类 fallback。
/// detail-impl 阶段填充。
pub struct DefaultCardDetector {
    // 在 detail-impl 阶段加入：
    //   template_matcher: TemplateMatcher,
    //   onnx_classifier: Arc<dyn CardClassifier>,
}

#[async_trait]
impl CardDetector for DefaultCardDetector {
    async fn detect(
        &self,
        _hole_rois: &[Rect; 2],
        _community_rois: &[Rect; 5],
        _frame: &Frame,
    ) -> Result<CardDetectionResult, TfError> {
        todo!("DefaultCardDetector::detect")
    }

    async fn detect_single(&self, _roi: &Rect, _frame: &Frame) -> Result<Option<Card>, TfError> {
        todo!("DefaultCardDetector::detect_single")
    }

    fn is_face_up_card(&self, _roi: &Frame) -> bool {
        todo!("DefaultCardDetector::is_face_up_card")
    }
}
