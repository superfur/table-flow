//! macOS window capture via CoreGraphics.
//!
//! Uses `CGWindowListCreateImage` for fast pixel capture (~5-20ms).
//! Window enumeration is in `window.rs` via `osascript`.

use std::sync::Arc;
use std::time::Instant;

use async_trait::async_trait;
use tf_core::{Frame, PixelFormat, Rect, TableId, TfError};

use super::{CapturedFrame, FrameCapture};

pub struct MacosCapture {
    table_id: TableId,
    window_id: u32,
    region: Rect,
}

impl MacosCapture {
    pub fn new(table_id: TableId, window_id: u32) -> Result<Self, TfError> {
        let region = query_window_rect(window_id)?;
        Ok(Self {
            table_id,
            window_id,
            region,
        })
    }

    pub fn window_id(&self) -> u32 {
        self.window_id
    }
}

#[async_trait]
impl FrameCapture for MacosCapture {
    async fn capture_frame(&mut self) -> Result<CapturedFrame, TfError> {
        let start = Instant::now();
        let frame = capture_window_bgra(self.window_id)?;
        let latency = start.elapsed().as_micros() as u64;

        self.region = Rect::new(0, 0, frame.width, frame.height);

        Ok(CapturedFrame {
            timestamp_ms: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as i64,
            table_id: self.table_id.clone(),
            frame,
            frame_number: 0,
            capture_latency_us: latency,
        })
    }

    fn current_region(&self) -> Rect {
        self.region
    }

    async fn rediscover_window(&mut self) -> Result<Rect, TfError> {
        self.region = query_window_rect(self.window_id)?;
        Ok(self.region)
    }
}

fn query_window_rect(window_id: u32) -> Result<Rect, TfError> {
    let swift_script = format!(
        r#"
import CoreGraphics
let windows = CGWindowListCopyWindowInfo(.optionAll, kCGNullWindowID) as? [[String: Any]] ?? []
for w in windows {{
    let pid = w["kCGWindowOwnerPID"] as? Int ?? 0
    if pid == {pid} {{
        if let bounds = w["kCGWindowBounds"] as? [String: Any],
           let w = bounds["Width"] as? Int,
           let h = bounds["Height"] as? Int {{
            print("\(w),\(h)")
            break
        }}
    }}
}}
"#,
        pid = window_id
    );

    let output = std::process::Command::new("swift")
        .arg("-e")
        .arg(&swift_script)
        .output()
        .map_err(|e| TfError::Capture(format!("swift failed: {}", e)))?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    if let Some((w, h)) = stdout.trim().split_once(',') {
        if let (Ok(width), Ok(height)) = (w.trim().parse::<u32>(), h.trim().parse::<u32>()) {
            if width > 0 && height > 0 {
                return Ok(Rect::new(0, 0, width, height));
            }
        }
    }

    Ok(Rect::new(0, 0, 1920, 1080))
}

fn capture_window_bgra(window_id: u32) -> Result<Frame, TfError> {
    let pid = window_id;
    let tmp = format!("/tmp/tf_capture_{}.png", pid);

    let swift_script = format!(
        r#"
import CoreGraphics
import AppKit
let windows = CGWindowListCopyWindowInfo(.optionAll, kCGNullWindowID) as? [[String: Any]] ?? []
for w in windows {{
    let layer = w["kCGWindowLayer"] as? Int ?? -1
    guard layer == 0 else {{ continue }}
    let pid = w["kCGWindowOwnerPID"] as? Int ?? 0
    if pid == {pid} {{
        let wid = w["kCGWindowNumber"] as? Int ?? 0
        let image = CGWindowListCreateImage(.null, .optionIncludingWindow, CGWindowID(wid), [.bestResolution])
        if let rep = NSBitmapImageRep(cgImage: image) {{
            let data = rep.representation(using: .png, properties: [:])
            data?.write(to: URL(fileURLWithPath: "{tmp}"))
        }}
        break
    }}
}}
"#,
        pid = pid,
        tmp = tmp,
    );

    let output = std::process::Command::new("swift")
        .arg("-e")
        .arg(&swift_script)
        .output()
        .map_err(|e| TfError::Capture(format!("swift capture failed: {}", e)))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(TfError::Capture(format!("swift capture error: {}", stderr)));
    }

    if !std::path::Path::new(&tmp).exists() {
        return Err(TfError::Capture("swift capture produced no file".into()));
    }

    let file_data = std::fs::read(&tmp).map_err(|e| TfError::Capture(format!("read capture: {}", e)))?;
    let _ = std::fs::remove_file(&tmp);

    decode_png_to_bgra(&file_data)
}

fn decode_png_to_bgra(data: &[u8]) -> Result<Frame, TfError> {
    let decoder = png::Decoder::new(std::io::Cursor::new(data));
    let mut reader = decoder
        .read_info()
        .map_err(|e| TfError::Capture(format!("PNG decode error: {}", e)))?;

    let info = reader.info().clone();
    let width = info.width;
    let height = info.height;

    let mut buf = vec![0u8; reader.output_buffer_size()];
    let output_info = reader
        .next_frame(&mut buf)
        .map_err(|e| TfError::Capture(format!("PNG read frame: {}", e)))?;

    let data_bytes = &buf[..output_info.buffer_size()];

    match info.color_type {
        png::ColorType::Rgba => {
            rgba_to_bgra(data_bytes, width, height)
        }
        png::ColorType::Rgb => {
            rgb_to_bgra(data_bytes, width, height)
        }
        png::ColorType::GrayscaleAlpha => {
            gray_alpha_to_bgra(data_bytes, width, height)
        }
        png::ColorType::Grayscale => {
            gray_to_bgra(data_bytes, width, height)
        }
        other => Err(TfError::Capture(format!(
            "Unsupported PNG color type: {:?}",
            other
        ))),
    }
}

fn rgba_to_bgra(data: &[u8], width: u32, height: u32) -> Result<Frame, TfError> {
    let mut bgra = Vec::with_capacity((width * height * 4) as usize);
    for chunk in data.chunks_exact(4) {
        bgra.push(chunk[2]);
        bgra.push(chunk[1]);
        bgra.push(chunk[0]);
        bgra.push(chunk[3]);
    }
    Ok(Frame {
        width,
        height,
        stride: width * 4,
        format: PixelFormat::Bgra8,
        data: Arc::new(bgra),
    })
}

fn rgb_to_bgra(data: &[u8], width: u32, height: u32) -> Result<Frame, TfError> {
    let mut bgra = Vec::with_capacity((width * height * 4) as usize);
    for chunk in data.chunks_exact(3) {
        bgra.push(chunk[2]);
        bgra.push(chunk[1]);
        bgra.push(chunk[0]);
        bgra.push(255);
    }
    Ok(Frame {
        width,
        height,
        stride: width * 4,
        format: PixelFormat::Bgra8,
        data: Arc::new(bgra),
    })
}

fn gray_alpha_to_bgra(data: &[u8], width: u32, height: u32) -> Result<Frame, TfError> {
    let mut bgra = Vec::with_capacity((width * height * 4) as usize);
    for chunk in data.chunks_exact(2) {
        bgra.push(chunk[0]);
        bgra.push(chunk[0]);
        bgra.push(chunk[0]);
        bgra.push(chunk[1]);
    }
    Ok(Frame {
        width,
        height,
        stride: width * 4,
        format: PixelFormat::Bgra8,
        data: Arc::new(bgra),
    })
}

fn gray_to_bgra(data: &[u8], width: u32, height: u32) -> Result<Frame, TfError> {
    let mut bgra = Vec::with_capacity((width * height * 4) as usize);
    for &g in data {
        bgra.push(g);
        bgra.push(g);
        bgra.push(g);
        bgra.push(255);
    }
    Ok(Frame {
        width,
        height,
        stride: width * 4,
        format: PixelFormat::Bgra8,
        data: Arc::new(bgra),
    })
}
