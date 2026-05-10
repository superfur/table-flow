//! 桌面区域校准 / Profile 加载与匹配

pub mod auto;

pub use auto::*;

use std::path::Path;

use tf_core::{CalibrationProfile, TfError};

pub fn load_profiles(dir: &Path) -> Result<Vec<CalibrationProfile>, TfError> {
    if !dir.is_dir() {
        return Err(TfError::Config(format!(
            "Calibration profiles directory not found: {}",
            dir.display()
        )));
    }

    let mut profiles = Vec::new();
    let entries = std::fs::read_dir(dir).map_err(|e| {
        TfError::Config(format!(
            "Failed to read profiles directory {}: {}",
            dir.display(),
            e
        ))
    })?;

    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) == Some("json") {
            let content = std::fs::read_to_string(&path).map_err(|e| {
                TfError::Config(format!(
                    "Failed to read profile {}: {}",
                    path.display(),
                    e
                ))
            })?;

            match serde_json::from_str::<CalibrationProfile>(&content) {
                Ok(profile) => profiles.push(profile),
                Err(e) => {
                    tracing::warn!("Skipping invalid profile {}: {}", path.display(), e);
                }
            }
        }
    }

    Ok(profiles)
}

pub fn match_profile<'a>(
    profiles: &'a [CalibrationProfile],
    window_title: &str,
    felt_color_hint: Option<(u8, u8, u8)>,
) -> Option<&'a CalibrationProfile> {
    let mut best: Option<&'a CalibrationProfile> = None;
    let mut best_score: f64 = 0.0;

    for profile in profiles {
        let mut score: f64 = 0.0;

        if let Ok(re) = regex::Regex::new(&profile.client_signature.window_title_regex) {
            if re.is_match(window_title) {
                score += 1.0;
            }
        }

        if let (Some((pr, pg, pb)), Some((hr, hg, hb))) =
            (felt_color_hint, profile.client_signature.felt_color_hint)
        {
            let dist = ((pr as f64 - hr as f64).powi(2)
                + (pg as f64 - hg as f64).powi(2)
                + (pb as f64 - hb as f64).powi(2))
            .sqrt();
            let color_sim = 1.0 - (dist / 441.67).min(1.0);
            score += color_sim * 0.5;
        }

        if score > best_score {
            best_score = score;
            best = Some(profile);
        }
    }

    best
}

#[cfg(test)]
mod tests {
    use super::*;
    use tf_core::{
        BlindsInfo, ClientSignature, DigitOcrRegions, NormalizedRect, SeatCalibration, SeatId,
        TableCalibration,
    };
    use std::io::Write;

    fn make_profile(id: &str, regex: &str) -> CalibrationProfile {
        CalibrationProfile {
            profile_id: id.to_string(),
            theme_id: "test".to_string(),
            client_signature: ClientSignature {
                window_title_regex: regex.to_string(),
                window_class: None,
                felt_color_hint: Some((0, 128, 0)),
            },
            calibration: TableCalibration {
                resolution: (1920, 1080),
                hole_card_positions: [
                    NormalizedRect::new(0.1, 0.5, 0.05, 0.08),
                    NormalizedRect::new(0.17, 0.5, 0.05, 0.08),
                ],
                community_card_positions: [
                    NormalizedRect::new(0.35, 0.4, 0.05, 0.08),
                    NormalizedRect::new(0.42, 0.4, 0.05, 0.08),
                    NormalizedRect::new(0.49, 0.4, 0.05, 0.08),
                    NormalizedRect::new(0.56, 0.4, 0.05, 0.08),
                    NormalizedRect::new(0.63, 0.4, 0.05, 0.08),
                ],
                pot_position: NormalizedRect::new(0.45, 0.3, 0.1, 0.04),
                seat_positions: vec![SeatCalibration {
                    seat_id: SeatId::new(0),
                    seat_region: NormalizedRect::new(0.0, 0.0, 0.1, 0.1),
                    stack_region: NormalizedRect::new(0.0, 0.08, 0.1, 0.03),
                    bet_region: NormalizedRect::new(0.05, 0.12, 0.05, 0.03),
                    avatar_region: NormalizedRect::new(0.0, 0.0, 0.05, 0.05),
                    card_region: None,
                }],
                dealer_button_region: NormalizedRect::new(0.3, 0.35, 0.03, 0.03),
                action_button_regions: [
                    NormalizedRect::new(0.4, 0.85, 0.08, 0.04),
                    NormalizedRect::new(0.5, 0.85, 0.08, 0.04),
                    NormalizedRect::new(0.6, 0.85, 0.08, 0.04),
                    NormalizedRect::new(0.7, 0.85, 0.08, 0.04),
                ],
                hero_seat: Some(SeatId::new(0)),
                blinds: BlindsInfo::default(),
                digit_ocr_regions: DigitOcrRegions::default(),
                theme_id: "test".to_string(),
            },
        }
    }

    #[test]
    fn test_load_profiles_nonexistent() {
        let result = load_profiles(Path::new("/nonexistent"));
        assert!(result.is_err());
    }

    #[test]
    fn test_load_profiles_from_dir() {
        let dir = std::env::temp_dir().join("tf_test_profiles");
        std::fs::create_dir_all(&dir).unwrap();

        let profile = make_profile("test1", r".*PokerStars.*");
        let json = serde_json::to_string_pretty(&profile).unwrap();
        let path = dir.join("test1.json");
        let mut f = std::fs::File::create(&path).unwrap();
        f.write_all(json.as_bytes()).unwrap();

        let profiles = load_profiles(&dir).unwrap();
        assert_eq!(profiles.len(), 1);
        assert_eq!(profiles[0].profile_id, "test1");

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn test_match_profile_by_title() {
        let profiles = vec![
            make_profile("pokerstars", r".*PokerStars.*"),
            make_profile("ggpoker", r".*GGPoker.*"),
        ];
        let result = match_profile(&profiles, "PokerStars Table #123", None);
        assert!(result.is_some());
        assert_eq!(result.unwrap().profile_id, "pokerstars");
    }

    #[test]
    fn test_match_profile_no_match() {
        let profiles = vec![make_profile("pokerstars", r".*PokerStars.*")];
        let result = match_profile(&profiles, "Unknown Window", None);
        assert!(result.is_none());
    }

    #[test]
    fn test_match_profile_with_color_hint() {
        let profiles = vec![make_profile("pokerstars", r".*Poker.*")];
        let result = match_profile(&profiles, "Poker Table", Some((0, 128, 0)));
        assert!(result.is_some());
    }
}
