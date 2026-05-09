//! FPS limiter（避免捕获过快浪费 CPU）

use std::time::{Duration, Instant};

pub struct FpsLimiter {
    target_fps: u32,
    min_interval: Duration,
    last_capture: Option<Instant>,
}

impl FpsLimiter {
    pub fn new(target_fps: u32) -> Self {
        let min_interval = if target_fps == 0 {
            Duration::ZERO
        } else {
            Duration::from_secs_f64(1.0 / target_fps as f64)
        };
        Self {
            target_fps,
            min_interval,
            last_capture: None,
        }
    }

    /// 异步等到下一次可以捕获的时刻。
    /// TODO(detail-impl): 用 tokio::time::sleep_until 实现，避免 busy-wait
    pub async fn wait(&mut self) {
        todo!("FpsLimiter::wait")
    }

    pub fn target_fps(&self) -> u32 {
        self.target_fps
    }

    pub fn set_target_fps(&mut self, fps: u32) {
        self.target_fps = fps;
        self.min_interval = if fps == 0 {
            Duration::ZERO
        } else {
            Duration::from_secs_f64(1.0 / fps as f64)
        };
    }
}
