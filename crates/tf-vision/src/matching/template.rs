//! 模板匹配引擎
//!
//! MVP 实现：归一化互相关（NCC）模板匹配。
//! 没有真正的 OpenCV，用像素亮度做滑动窗口匹配。

use std::collections::HashMap;
use std::path::Path;

use tf_core::{Frame, PixelFormat, TfError};

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

    pub fn load_directory(&mut self, dir: &Path) -> Result<usize, TfError> {
        if !dir.is_dir() {
            return Err(TfError::Config(format!(
                "Template directory not found: {}",
                dir.display()
            )));
        }

        let mut count = 0usize;
        let entries = std::fs::read_dir(dir).map_err(|e| {
            TfError::Config(format!("Failed to read directory {}: {}", dir.display(), e))
        })?;

        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("png") {
                if let Some(id) = path.file_stem().and_then(|s| s.to_str()) {
                    if let Ok(frame) = load_png_as_frame(&path) {
                        self.templates.insert(id.to_string(), frame);
                        count += 1;
                    }
                }
            }
        }

        Ok(count)
    }

    pub fn match_one(
        &self,
        template_id: &str,
        frame: &Frame,
    ) -> Result<Option<TemplateMatch>, TfError> {
        let tmpl = match self.templates.get(template_id) {
            Some(t) => t,
            None => return Ok(None),
        };

        if tmpl.width > frame.width || tmpl.height > frame.height {
            return Ok(None);
        }

        let result = ncc_search(frame, tmpl);
        if result.confidence >= self.min_confidence {
            Ok(Some(TemplateMatch {
                template_id: template_id.to_string(),
                confidence: result.confidence,
                location: (result.x as i32, result.y as i32),
            }))
        } else {
            Ok(None)
        }
    }

    pub fn match_best(&self, frame: &Frame) -> Result<Option<TemplateMatch>, TfError> {
        let mut best: Option<TemplateMatch> = None;

        for id in self.templates.keys() {
            if let Some(m) = self.match_one(id, frame)? {
                match &best {
                    Some(b) if b.confidence >= m.confidence => {}
                    _ => best = Some(m),
                }
            }
        }

        Ok(best)
    }
}

struct NccResult {
    x: usize,
    y: usize,
    confidence: f32,
}

fn ncc_search(frame: &Frame, tmpl: &Frame) -> NccResult {
    let fw = frame.width as usize;
    let fh = frame.height as usize;
    let tw = tmpl.width as usize;
    let th = tmpl.height as usize;
    let fc = channels(frame.format);
    let tc = channels(tmpl.format);
    let fstride = frame.stride as usize;
    let tstride = tmpl.stride as usize;

    let mut best_score: f32 = -1.0;
    let mut best_x: usize = 0;
    let mut best_y: usize = 0;

    let step = ((fw - tw + 1) / 8).max(1);
    let ystep = ((fh - th + 1) / 8).max(1);

    for y in (0..=(fh - th)).step_by(ystep) {
        for x in (0..=(fw - tw)).step_by(step) {
            let score = compute_ncc(frame, tmpl, x, y, fc, tc, fstride, tstride);
            if score > best_score {
                best_score = score;
                best_x = x;
                best_y = y;
            }
        }
    }

    NccResult {
        x: best_x,
        y: best_y,
        confidence: best_score.max(0.0),
    }
}

fn compute_ncc(
    frame: &Frame,
    tmpl: &Frame,
    ox: usize,
    oy: usize,
    fc: usize,
    tc: usize,
    fstride: usize,
    tstride: usize,
) -> f32 {
    let tw = tmpl.width as usize;
    let th = tmpl.height as usize;
    let n = (tw * th) as f64;

    let mut sum_f: f64 = 0.0;
    let mut sum_t: f64 = 0.0;
    let mut sum_ft: f64 = 0.0;
    let mut sum_f2: f64 = 0.0;
    let mut sum_t2: f64 = 0.0;

    let sample_step = (tw / 4).max(1);

    for ty in (0..th).step_by(2) {
        for tx in (0..tw).step_by(sample_step) {
            let foff = (oy + ty) * fstride + (ox + tx) * fc;
            let toff = ty * tstride + tx * tc;

            let fv = if foff < frame.data.len() {
                frame.data[foff] as f64
            } else {
                0.0
            };
            let tv = if toff < tmpl.data.len() {
                tmpl.data[toff] as f64
            } else {
                0.0
            };

            sum_f += fv;
            sum_t += tv;
            sum_ft += fv * tv;
            sum_f2 += fv * fv;
            sum_t2 += tv * tv;
        }
    }

    let denom = ((n * sum_f2 - sum_f * sum_f) * (n * sum_t2 - sum_t * sum_t)).sqrt();
    if denom == 0.0 {
        return 0.0;
    }

    ((n * sum_ft - sum_f * sum_t) / denom) as f32
}

fn load_png_as_frame(path: &Path) -> Result<Frame, std::io::Error> {
    let data = std::fs::read(path)?;
    if data.len() < 24 {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "File too small",
        ));
    }

    let w = u32::from_be_bytes([data[16], data[17], data[18], data[19]]);
    let h = u32::from_be_bytes([data[20], data[21], data[22], data[23]]);

    Ok(Frame {
        width: w,
        height: h,
        stride: w * 3,
        format: PixelFormat::Rgb8,
        data: std::sync::Arc::new(data),
    })
}

fn channels(fmt: PixelFormat) -> usize {
    match fmt {
        PixelFormat::Gray8 => 1,
        PixelFormat::Bgr8 | PixelFormat::Rgb8 => 3,
        PixelFormat::Bgra8 => 4,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    fn make_frame(w: u32, h: u32, fill: u8) -> Frame {
        Frame {
            width: w,
            height: h,
            stride: w * 3,
            format: PixelFormat::Rgb8,
            data: Arc::new(vec![fill; (w * h * 3) as usize]),
        }
    }

    #[test]
    fn test_new_matcher() {
        let m = TemplateMatcher::new(0.8);
        assert!(m.templates.is_empty());
        assert_eq!(m.min_confidence, 0.8);
    }

    #[test]
    fn test_match_one_missing_template() {
        let m = TemplateMatcher::new(0.5);
        let frame = make_frame(640, 480, 128);
        let result = m.match_one("nonexistent", &frame).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_match_best_empty() {
        let m = TemplateMatcher::new(0.5);
        let frame = make_frame(640, 480, 128);
        let result = m.match_best(&frame).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_match_one_template_larger_than_frame() {
        let mut m = TemplateMatcher::new(0.5);
        m.templates.insert(
            "big".to_string(),
            make_frame(800, 600, 128),
        );
        let frame = make_frame(640, 480, 128);
        let result = m.match_one("big", &frame).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_load_directory_nonexistent() {
        let mut m = TemplateMatcher::new(0.5);
        let result = m.load_directory(Path::new("/nonexistent/path"));
        assert!(result.is_err());
    }

    #[test]
    fn test_match_one_with_template() {
        let mut m = TemplateMatcher::new(0.0);
        let tmpl = make_frame(30, 30, 200);
        m.templates.insert("test".to_string(), tmpl);
        let frame = make_frame(640, 480, 200);
        let result = m.match_one("test", &frame).unwrap();
        assert!(result.is_some());
        let r = result.unwrap();
        assert_eq!(r.template_id, "test");
        assert!(r.confidence > 0.0);
    }
}
