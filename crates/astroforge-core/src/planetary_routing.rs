use crate::image::F32Image;
use crate::ingest::FrameInfo;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TargetType {
    DeepSky,
    Planetary,
    Lunar,
}

pub fn route_session(
    frames: &[FrameInfo],
    exptime_threshold: f64,
    frame_count_threshold: usize,
) -> TargetType {
    let avg_exptime = if frames.is_empty() {
        0.0
    } else {
        let sum: f64 = frames.iter().filter_map(|f| f.exptime).sum();
        sum / frames.len() as f64
    };

    let count = frames.len();

    if avg_exptime < exptime_threshold && count > frame_count_threshold {
        let has_lunar_name = frames.iter().any(|f| {
            f.path
                .file_name()
                .and_then(|n| n.to_str())
                .map(|s| s.to_uppercase())
                .map(|s| s.contains("MOON") || s.contains("LUNAR"))
                .unwrap_or(false)
        });
        if has_lunar_name {
            TargetType::Lunar
        } else {
            TargetType::Planetary
        }
    } else if avg_exptime > 10.0 && count < 500 {
        TargetType::DeepSky
    } else {
        TargetType::DeepSky
    }
}

pub fn route_session_prompt(frames: &[FrameInfo]) -> Option<TargetType> {
    let avg_exptime = if frames.is_empty() {
        return None;
    } else {
        let sum: f64 = frames.iter().filter_map(|f| f.exptime).sum();
        sum / frames.len() as f64
    };

    let count = frames.len();

    if avg_exptime >= 2.0 && avg_exptime <= 10.0 {
        return None;
    }
    if avg_exptime < 2.0 && count <= 500 {
        return None;
    }

    Some(route_session(frames, 2.0, 500))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ingest::FrameType;
    use std::path::PathBuf;

    fn make_frames(count: usize, exptime: f64) -> Vec<FrameInfo> {
        (0..count)
            .map(|i| FrameInfo {
                path: PathBuf::from(format!("frame_{:05}.fits", i)),
                frame_type: FrameType::Light,
                exptime: Some(exptime),
                filter: None,
                date_obs: None,
                ccd_temp: None,
                width: Some(640),
                height: Some(480),
                binning: Some(1),
                anomalies: vec![],
            })
            .collect()
    }

    #[test]
    fn test_route_planetary() {
        let frames = make_frames(1000, 0.01);
        assert_eq!(route_session(&frames, 2.0, 500), TargetType::Planetary);
    }

    #[test]
    fn test_route_deep_sky() {
        let frames = make_frames(30, 120.0);
        assert_eq!(route_session(&frames, 2.0, 500), TargetType::DeepSky);
    }

    #[test]
    fn test_route_lunar() {
        let mut frames = make_frames(2000, 0.01);
        for f in &mut frames {
            f.path = PathBuf::from(format!("moon_{:05}.fits", 0));
        }
        assert_eq!(route_session(&frames, 2.0, 500), TargetType::Lunar);
    }

    #[test]
    fn test_route_ambiguous_returns_none() {
        let frames = make_frames(100, 5.0);
        assert!(route_session_prompt(&frames).is_none());
    }

    #[test]
    fn test_route_clear_planetary() {
        let frames = make_frames(1000, 0.01);
        assert_eq!(route_session_prompt(&frames), Some(TargetType::Planetary));
    }
}
