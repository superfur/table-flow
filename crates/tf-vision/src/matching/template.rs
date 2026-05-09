//! 模板匹配引擎

use std::collections::HashMap;

use tf_core::{Frame, TfError};

#[derive(Debug, Clone)]
pub struct TemplateMatch {
    pub template_id: String,
    pub confidence: f32,
    pub location: (i32, i32),
}

pub struct TemplateMatcher {
    pub templates: HashMap<String, Frame>,
    pub min_confidence: f32,
}

impl TemplateMatcher {
    pub fn new(min_confidence: f32) -> Self {
        Self {
            templates: HashMap::new(),
            min_confidence,
        }
    }

    /// 从磁盘批量加载一组模板（同一目录下的 PNG）
    /// TODO(detail-impl)
    pub fn load_directory(&mut self, _dir: &std::path::Path) -> Result<usize, TfError> {
        todo!("TemplateMatcher::load_directory")
    }

    /// 在 frame 中匹配某一个具体模板
    /// TODO(detail-impl): 多尺度（0.9–1.1x）+ TM_CCOEFF_NORMED
    pub fn match_one(
        &self,
        _template_id: &str,
        _frame: &Frame,
    ) -> Result<Option<TemplateMatch>, TfError> {
        todo!("TemplateMatcher::match_one")
    }

    /// 在 frame 中找出所有 templates 中置信度最高的一个
    pub fn match_best(&self, _frame: &Frame) -> Result<Option<TemplateMatch>, TfError> {
        todo!("TemplateMatcher::match_best")
    }
}
