//! 把各个 detector 的输出合并成 `ExtractedFeatures`

use tf_core::TableId;

use crate::features::{ExtractedFeatures, RawFeatures};

pub struct FeatureAggregator {
    pub table_id: TableId,
}

impl FeatureAggregator {
    pub fn new(table_id: TableId) -> Self {
        Self { table_id }
    }

    /// 合并 + 推断 street（基于公共牌数量）。
    /// TODO(detail-impl)
    pub fn merge(&self, _raw: RawFeatures) -> ExtractedFeatures {
        todo!("FeatureAggregator::merge")
    }
}
