//! Window enumeration / tracking

use serde::{Deserialize, Serialize};

use tf_core::{Rect, TfError};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowInfo {
    pub handle: u64,
    pub title: String,
    pub class_name: Option<String>,
    pub bounds: Rect,
}

/// Mock implementation for macOS / testing.
/// Production: Win32 EnumWindows + GetWindowText + GetWindowRect.
pub fn enumerate_windows(title_regex: &str) -> Result<Vec<WindowInfo>, TfError> {
    let re = regex::Regex::new(title_regex)
        .map_err(|e| TfError::Vision(format!("invalid regex '{}': {}", title_regex, e)))?;
    let all = enumerate_all_windows()?;
    Ok(all.into_iter().filter(|w| re.is_match(&w.title)).collect())
}

fn enumerate_all_windows() -> Result<Vec<WindowInfo>, TfError> {
    #[cfg(target_os = "macos")]
    {
        macos_list_windows()
    }
    #[cfg(target_os = "windows")]
    {
        crate::capture::window::win32_enum_windows()
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        Ok(vec![])
    }
}

pub fn get_window_bounds(_handle: u64) -> Result<Rect, TfError> {
    #[cfg(target_os = "macos")]
    {
        macos_list_windows()
            .unwrap_or_default()
            .into_iter()
            .find(|w| w.handle == _handle)
            .map(|w| w.bounds)
            .ok_or_else(|| TfError::WindowNotFound(format!("Window {} not found", _handle)))
    }
    #[cfg(target_os = "windows")]
    {
        crate::capture::window::win32_get_bounds(_handle)
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        Err(TfError::WindowNotFound(
            "Window enumeration only available on macOS/Windows".into(),
        ))
    }
}

#[cfg(target_os = "macos")]
fn macos_list_windows() -> Result<Vec<WindowInfo>, TfError> {
    let swift_script = r#"
import CoreGraphics
let windows = CGWindowListCopyWindowInfo(.optionAll, kCGNullWindowID) as? [[String: Any]] ?? []
for w in windows {
    let layer = w["kCGWindowLayer"] as? Int ?? -1
    guard layer == 0 else { continue }
    let owner = w["kCGWindowOwnerName"] as? String ?? ""
    let name = w["kCGWindowName"] as? String ?? ""
    let pid = w["kCGWindowOwnerPID"] as? Int ?? 0
    if !name.isEmpty {
        print("\(pid),\(owner) - \(name)")
    }
}
"#;

    let output = std::process::Command::new("swift")
        .arg("-e")
        .arg(swift_script)
        .output()
        .map_err(|e| TfError::Capture(format!("swift failed: {}", e)))?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut windows = Vec::new();

    for line in stdout.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if let Some((pid_str, rest)) = line.split_once(',') {
            if let Ok(pid) = pid_str.trim().parse::<u64>() {
                let title = rest.trim();
                if !title.is_empty() {
                    windows.push(WindowInfo {
                        handle: pid,
                        title: title.to_string(),
                        class_name: None,
                        bounds: Rect::new(0, 0, 0, 0),
                    });
                }
            }
        }
    }

    Ok(windows)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_enumerate_invalid_regex() {
        let result = enumerate_windows("[invalid");
        assert!(result.is_err());
    }

    #[test]
    fn test_enumerate_valid_regex() {
        let result = enumerate_windows(".*");
        assert!(result.is_ok());
    }

    #[test]
    fn test_get_window_bounds_non_windows() {
        #[cfg(not(target_os = "windows"))]
        {
            let result = get_window_bounds(0);
            assert!(result.is_err());
        }
    }
}
