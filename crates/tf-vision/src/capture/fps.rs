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

    pub async fn wait(&mut self) {
        if self.min_interval.is_zero() {
            return;
        }
        if let Some(last) = self.last_capture {
            let elapsed = last.elapsed();
            if elapsed < self.min_interval {
                tokio::time::sleep(self.min_interval - elapsed).await;
            }
        }
        self.last_capture = Some(Instant::now());
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_fps_limiter() {
        let limiter = FpsLimiter::new(30);
        assert_eq!(limiter.target_fps(), 30);
        assert!(!limiter.min_interval.is_zero());
    }

    #[test]
    fn test_zero_fps() {
        let limiter = FpsLimiter::new(0);
        assert!(limiter.min_interval.is_zero());
    }

    #[test]
    fn test_set_target_fps() {
        let mut limiter = FpsLimiter::new(30);
        limiter.set_target_fps(60);
        assert_eq!(limiter.target_fps(), 60);
    }

    #[tokio::test]
    async fn test_wait_first_call() {
        let mut limiter = FpsLimiter::new(1000);
        limiter.wait().await;
        assert!(limiter.last_capture.is_some());
    }
}
