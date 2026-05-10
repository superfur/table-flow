//! 共用的张量预处理 / 后处理原语。
//!
//! 纯字节级转换，不依赖 ONNX / OpenCV。

use tf_core::{Frame, PixelFormat, TfError};

/// BGRA → RGB 字节转换（输入：每像素 4 字节 BGRA，输出：每像素 3 字节 RGB）
pub fn bgra_to_rgb(frame: &Frame) -> Result<Frame, TfError> {
    if frame.format != PixelFormat::Bgra8 {
        return Err(TfError::Vision(format!(
            "expected BGRA8, got {:?}",
            frame.format
        )));
    }
    let pixel_count = frame.width as usize * frame.height as usize;
    let src = &frame.data;
    if src.len() < pixel_count * 4 {
        return Err(TfError::Vision("BGRA frame data too short".into()));
    }
    let mut dst = Vec::with_capacity(pixel_count * 3);
    for i in 0..pixel_count {
        let off = i * 4;
        // BGRA → RGB: swap B↔R
        dst.push(src[off + 2]); // R
        dst.push(src[off + 1]); // G
        dst.push(src[off]);     // B
    }
    Ok(Frame {
        width: frame.width,
        height: frame.height,
        stride: frame.width * 3,
        format: PixelFormat::Rgb8,
        data: std::sync::Arc::new(dst),
    })
}

/// 把 Frame 转为灰度（支持 BGRA8 / BGR8 / RGB8 → Gray8）
pub fn to_grayscale(frame: &Frame) -> Result<Frame, TfError> {
    let pixel_count = frame.width as usize * frame.height as usize;
    let src = &frame.data;
    let mut gray = Vec::with_capacity(pixel_count);

    match frame.format {
        PixelFormat::Bgra8 => {
            if src.len() < pixel_count * 4 {
                return Err(TfError::Vision("frame data too short".into()));
            }
            for i in 0..pixel_count {
                let off = i * 4;
                let b = src[off] as f32;
                let g = src[off + 1] as f32;
                let r = src[off + 2] as f32;
                gray.push((0.114 * b + 0.587 * g + 0.299 * r) as u8);
            }
        }
        PixelFormat::Bgr8 => {
            if src.len() < pixel_count * 3 {
                return Err(TfError::Vision("frame data too short".into()));
            }
            for i in 0..pixel_count {
                let off = i * 3;
                let b = src[off] as f32;
                let g = src[off + 1] as f32;
                let r = src[off + 2] as f32;
                gray.push((0.114 * b + 0.587 * g + 0.299 * r) as u8);
            }
        }
        PixelFormat::Rgb8 => {
            if src.len() < pixel_count * 3 {
                return Err(TfError::Vision("frame data too short".into()));
            }
            for i in 0..pixel_count {
                let off = i * 3;
                let r = src[off] as f32;
                let g = src[off + 1] as f32;
                let b = src[off + 2] as f32;
                gray.push((0.299 * r + 0.587 * g + 0.114 * b) as u8);
            }
        }
        PixelFormat::Gray8 => {
            gray = src.to_vec();
        }
    }

    Ok(Frame {
        width: frame.width,
        height: frame.height,
        stride: frame.width,
        format: PixelFormat::Gray8,
        data: std::sync::Arc::new(gray),
    })
}

/// 双线性插值 resize（通用，不依赖 OpenCV）
pub fn resize(frame: &Frame, target_w: u32, target_h: u32) -> Result<Frame, TfError> {
    let channels = match frame.format {
        PixelFormat::Gray8 => 1,
        PixelFormat::Bgr8 | PixelFormat::Rgb8 => 3,
        PixelFormat::Bgra8 => 4,
    };
    let src_w = frame.width as f64;
    let src_h = frame.height as f64;
    let dst_w = target_w as usize;
    let dst_h = target_h as usize;
    let src = &frame.data;

    let mut dst = Vec::with_capacity(dst_w * dst_h * channels);

    for y in 0..dst_h {
        let src_y = y as f64 * (src_h / target_h as f64);
        let y0 = (src_y.floor() as usize).min(frame.height as usize - 1);
        let y1 = (y0 + 1).min(frame.height as usize - 1);
        let fy = src_y - y0 as f64;

        for x in 0..dst_w {
            let src_x = x as f64 * (src_w / target_w as f64);
            let x0 = (src_x.floor() as usize).min(frame.width as usize - 1);
            let x1 = (x0 + 1).min(frame.width as usize - 1);
            let fx = src_x - x0 as f64;

            for c in 0..channels {
                let v00 = src[(y0 * frame.width as usize + x0) * channels + c] as f64;
                let v01 = src[(y0 * frame.width as usize + x1) * channels + c] as f64;
                let v10 = src[(y1 * frame.width as usize + x0) * channels + c] as f64;
                let v11 = src[(y1 * frame.width as usize + x1) * channels + c] as f64;

                let v = v00 * (1.0 - fx) * (1.0 - fy)
                    + v01 * fx * (1.0 - fy)
                    + v10 * (1.0 - fx) * fy
                    + v11 * fx * fy;
                dst.push(v.round().clamp(0.0, 255.0) as u8);
            }
        }
    }

    Ok(Frame {
        width: target_w,
        height: target_h,
        stride: target_w * channels as u32,
        format: frame.format,
        data: std::sync::Arc::new(dst),
    })
}

/// CTC 贪心解码（用于 PaddleOCR 输出 → 字符序列）
///
/// `logits`: `[T * num_classes]` 扁平数组，行优先（每帧 num_classes 个概率）
/// `num_classes`: 包含 blank 类的总类别数（blank index = 0）
/// `charset`: 可选字符映射表；`None` 时用默认 `0-9` + blank
///
/// 返回 (decoded_string, average_max_confidence)
pub fn ctc_greedy_decode(
    logits: &[f32],
    num_classes: usize,
    charset: Option<&[char]>,
) -> (String, f32) {
    let t_steps = logits.len() / num_classes;
    if t_steps == 0 || num_classes == 0 {
        return (String::new(), 0.0);
    }

    let default_charset: Vec<char> = std::iter::once('_')
        .chain('0'..='9')
        .collect();
    let charset = charset.unwrap_or(&default_charset);

    let mut result = String::new();
    let mut total_conf = 0.0f32;
    let mut prev_class = 0usize; // blank

    for t in 0..t_steps {
        let row = &logits[t * num_classes..(t + 1) * num_classes];
        let (best_class, best_val) = row
            .iter()
            .enumerate()
            .max_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(i, &v)| (i, v))
            .unwrap_or((0, 0.0));

        // softmax 近似：max_prob = exp(best_val) / sum(exp(all))
        let max_exp = (best_val - row.iter().copied().fold(f32::NEG_INFINITY, f32::max)).exp();
        let sum_exp: f32 = row
            .iter()
            .map(|&v| (v - row.iter().copied().fold(f32::NEG_INFINITY, f32::max)).exp())
            .sum();
        let conf = if sum_exp > 0.0 { max_exp / sum_exp } else { 0.0 };

        if best_class != 0 && best_class != prev_class {
            if let Some(&ch) = charset.get(best_class) {
                result.push(ch);
            }
        }
        prev_class = best_class;
        total_conf += conf;
    }

    let avg_conf = if t_steps > 0 {
        total_conf / t_steps as f32
    } else {
        0.0
    };
    (result, avg_conf)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_bgra_frame(w: u32, h: u32, fill: u8) -> Frame {
        let data = vec![fill; (w * h * 4) as usize];
        Frame {
            width: w,
            height: h,
            stride: w * 4,
            format: PixelFormat::Bgra8,
            data: std::sync::Arc::new(data),
        }
    }

    fn make_rgb_frame(w: u32, h: u32, r: u8, g: u8, b: u8) -> Frame {
        let mut data = Vec::with_capacity((w * h * 3) as usize);
        for _ in 0..(w * h) {
            data.push(r);
            data.push(g);
            data.push(b);
        }
        Frame {
            width: w,
            height: h,
            stride: w * 3,
            format: PixelFormat::Rgb8,
            data: std::sync::Arc::new(data),
        }
    }

    #[test]
    fn test_bgra_to_rgb_size() {
        let frame = make_bgra_frame(4, 3, 128);
        let rgb = bgra_to_rgb(&frame).unwrap();
        assert_eq!(rgb.format, PixelFormat::Rgb8);
        assert_eq!(rgb.data.len(), 4 * 3 * 3);
        assert_eq!(rgb.width, 4);
        assert_eq!(rgb.height, 3);
    }

    #[test]
    fn test_bgra_to_rgb_channel_order() {
        let mut data = vec![0u8; 8]; // 2 pixels BGRA
        data[0] = 10; // B
        data[1] = 20; // G
        data[2] = 30; // R
        data[3] = 255; // A
        data[4] = 40;
        data[5] = 50;
        data[6] = 60;
        data[7] = 254;
        let frame = Frame {
            width: 2,
            height: 1,
            stride: 8,
            format: PixelFormat::Bgra8,
            data: std::sync::Arc::new(data),
        };
        let rgb = bgra_to_rgb(&frame).unwrap();
        assert_eq!(rgb.data.as_slice(), &[30, 20, 10, 60, 50, 40]);
    }

    #[test]
    fn test_to_grayscale_from_rgb() {
        let frame = make_rgb_frame(2, 2, 100, 150, 200);
        let gray = to_grayscale(&frame).unwrap();
        assert_eq!(gray.format, PixelFormat::Gray8);
        assert_eq!(gray.data.len(), 4);
        // 0.299*100 + 0.587*150 + 0.114*200 ≈ 140
        let expected = (0.299 * 100.0 + 0.587 * 150.0 + 0.114 * 200.0) as u8;
        for &v in gray.data.iter() {
            assert!((v as i16 - expected as i16).abs() <= 1);
        }
    }

    #[test]
    fn test_to_grayscale_passthrough() {
        let data = vec![42u8; 9];
        let frame = Frame {
            width: 3,
            height: 3,
            stride: 3,
            format: PixelFormat::Gray8,
            data: std::sync::Arc::new(data),
        };
        let gray = to_grayscale(&frame).unwrap();
        assert_eq!(gray.data.as_slice(), &[42u8; 9]);
    }

    #[test]
    fn test_resize_dimensions() {
        let frame = make_rgb_frame(100, 100, 128, 128, 128);
        let resized = resize(&frame, 50, 50).unwrap();
        assert_eq!(resized.width, 50);
        assert_eq!(resized.height, 50);
        assert_eq!(resized.data.len(), 50 * 50 * 3);
    }

    #[test]
    fn test_resize_uniform_color() {
        let frame = make_rgb_frame(10, 10, 200, 100, 50);
        let resized = resize(&frame, 5, 5).unwrap();
        for chunk in resized.data.chunks(3) {
            assert_eq!(chunk[0], 200);
            assert_eq!(chunk[1], 100);
            assert_eq!(chunk[2], 50);
        }
    }

    #[test]
    fn test_ctc_greedy_decode_simple() {
        // 3 time steps, 12 classes (blank=0, 1-9='0'-'8', 10='9')
        let num_classes = 12;
        let mut logits = vec![0.0f32; 3 * num_classes];
        // step 0: class 1 ('0')
        logits[0 * num_classes + 1] = 10.0;
        // step 1: class 1 ('0') — duplicate, should collapse
        logits[1 * num_classes + 1] = 10.0;
        // step 2: class 5 ('4')
        logits[2 * num_classes + 5] = 10.0;

        let (text, conf) = ctc_greedy_decode(&logits, num_classes, None);
        assert_eq!(text, "04");
        assert!(conf > 0.9);
    }

    #[test]
    fn test_ctc_greedy_decode_blank_only() {
        let num_classes = 12;
        let mut logits = vec![0.0f32; 2 * num_classes];
        logits[0] = 10.0; // blank
        logits[num_classes] = 10.0; // blank

        let (text, _) = ctc_greedy_decode(&logits, num_classes, None);
        assert!(text.is_empty());
    }

    #[test]
    fn test_ctc_greedy_decode_with_charset() {
        let num_classes = 4;
        let charset: Vec<char> = vec!['_', 'A', 'B', 'C'];
        let mut logits = vec![0.0f32; 2 * num_classes];
        logits[1] = 10.0; // 'A'
        logits[num_classes + 3] = 10.0; // 'C'

        let (text, _) = ctc_greedy_decode(&logits, num_classes, Some(&charset));
        assert_eq!(text, "AC");
    }

    #[test]
    fn test_ctc_empty_input() {
        let (text, conf) = ctc_greedy_decode(&[], 10, None);
        assert!(text.is_empty());
        assert_eq!(conf, 0.0);
    }
}
