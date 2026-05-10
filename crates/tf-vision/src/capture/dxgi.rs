//! DXGI Desktop Duplication / Mock FrameCapture

use async_trait::async_trait;

use tf_core::{Frame, PixelFormat, Rect, TableId, TfError};

use super::{CapturedFrame, FrameCapture};

pub struct DxgiCapture {
    pub table_id: TableId,
    pub window_title_pattern: String,
    pub region: Rect,
}

impl DxgiCapture {
    pub fn new(table_id: TableId, window_title_pattern: String) -> Result<Self, TfError> {
        Ok(Self {
            table_id,
            window_title_pattern,
            region: Rect::new(0, 0, 1920, 1080),
        })
    }
}

#[async_trait]
impl FrameCapture for DxgiCapture {
    async fn capture_frame(&mut self) -> Result<CapturedFrame, TfError> {
        Err(TfError::Capture(
            "DxgiCapture only available on Windows".into(),
        ))
    }

    fn current_region(&self) -> Rect {
        self.region
    }

    async fn rediscover_window(&mut self) -> Result<Rect, TfError> {
        Err(TfError::Capture(
            "DxgiCapture only available on Windows".into(),
        ))
    }
}

/// Mock capture for testing — generates blank frames or loads from image data.
pub struct MockCapture {
    table_id: TableId,
    region: Rect,
    frames: Vec<Frame>,
    frame_index: usize,
}

impl MockCapture {
    pub fn new(table_id: TableId, region: Rect) -> Self {
        Self {
            table_id,
            region,
            frames: Vec::new(),
            frame_index: 0,
        }
    }

    pub fn with_frames(table_id: TableId, region: Rect, frames: Vec<Frame>) -> Self {
        Self {
            table_id,
            region,
            frames,
            frame_index: 0,
        }
    }

    pub fn blank(table_id: TableId, width: u32, height: u32) -> Self {
        let frame = Frame {
            width,
            height,
            stride: width * 4,
            format: PixelFormat::Bgra8,
            data: std::sync::Arc::new(vec![0u8; (width * height * 4) as usize]),
        };
        Self::with_frames(table_id, Rect::new(0, 0, width, height), vec![frame])
    }

    pub fn add_frame(&mut self, frame: Frame) {
        self.frames.push(frame);
    }
}

#[async_trait]
impl FrameCapture for MockCapture {
    async fn capture_frame(&mut self) -> Result<CapturedFrame, TfError> {
        let frame = if self.frames.is_empty() {
            Frame {
                width: self.region.width,
                height: self.region.height,
                stride: self.region.width * 4,
                format: PixelFormat::Bgra8,
                data: std::sync::Arc::new(vec![0u8; (self.region.width * self.region.height * 4) as usize]),
            }
        } else {
            let idx = self.frame_index % self.frames.len();
            self.frame_index += 1;
            self.frames[idx].clone()
        };

        let frame_number = self.frame_index as u64;

        Ok(CapturedFrame {
            timestamp_ms: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as i64,
            table_id: self.table_id.clone(),
            frame,
            frame_number,
            capture_latency_us: 0,
        })
    }

    fn current_region(&self) -> Rect {
        self.region
    }

    async fn rediscover_window(&mut self) -> Result<Rect, TfError> {
        Ok(self.region)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mock_capture_blank() {
        let mut cap = MockCapture::blank("test".into(), 100, 100);
        let frame = cap.capture_frame().await.unwrap();
        assert_eq!(frame.frame.width, 100);
        assert_eq!(frame.frame.height, 100);
        assert_eq!(frame.table_id, "test");
    }

    #[tokio::test]
    async fn test_mock_capture_cycle() {
        let f1 = Frame {
            width: 10, height: 10, stride: 40,
            format: PixelFormat::Bgra8,
            data: std::sync::Arc::new(vec![1u8; 400]),
        };
        let f2 = Frame {
            width: 10, height: 10, stride: 40,
            format: PixelFormat::Bgra8,
            data: std::sync::Arc::new(vec![2u8; 400]),
        };
        let mut cap = MockCapture::with_frames("test".into(), Rect::new(0, 0, 10, 10), vec![f1, f2]);

        let r1 = cap.capture_frame().await.unwrap();
        let r2 = cap.capture_frame().await.unwrap();
        let r3 = cap.capture_frame().await.unwrap();

        assert_eq!(r1.frame.data[0], 1);
        assert_eq!(r2.frame.data[0], 2);
        assert_eq!(r3.frame.data[0], 1);
    }
}
