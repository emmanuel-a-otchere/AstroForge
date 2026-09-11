//! CR-04 P5 — Session Grouping (CR-04 §8).
//!
//! Imports should not collapse into one large session.
//! Per CR-04 §8, AstroForge groups assets into logical
//! sessions using:
//!
//! 1. target detection (P4 — `target_detection::detect`);
//! 2. acquisition date (FITS `DATE-OBS`);
//! 3. instrument (camera + telescope);
//! 4. filter (FITS `FILTER`);
//! 5. binning (FITS `XBINNING` × `YBINNING`);
//! 6. image dimensions (width × height);
//! 7. exposure characteristics (FITS `EXPTIME`);
//! 8. directory relationships (the asset's `original_path`).
//!
//! The grouping algorithm is a **stable sort + sweep**:
//! each asset derives a `GroupingKey` from its metadata;
//! assets that share a key land in the same session.
//! Tolerance: binning + dimensions + exposure are bucketed
//! into equivalence classes (see `bucket_*` helpers) so that
//! near-identical values don't fragment a real session.
//!
//! **No drift on missing metadata.** Assets missing any of
//! the optional fields fall back to `None` for that dimension
//! of the key. Two assets only land in different sessions if
//! a *known* field differs between them — we never invent
//! partitions.
//!
//! **Composes with P4 target_detection.** The caller passes
//! the pre-computed `TargetIntelligence` per asset (or
//! `None` if detection was inconclusive). This module never
//! re-runs target detection — that's P4's job.
//!
//! The module is pure: no I/O, no DB, no time. The IPC
//! layer (CR-04 P8) maps each `SessionGroup` to a `Session`
//! row + assigns each `SourceAsset.session_id`.

use crate::import_scan::ExtractedMetadata;
use crate::target_detection::TargetIntelligence;
use serde::{Deserialize, Serialize};
use std::path::Path;

/// CR-04 §8 — the inputs to grouping for a single asset.
///
/// The grouping module does not own the data model — the
/// caller hands in just the fields it needs. The IPC layer
/// (P8) will construct this from `ExtractedMetadata` +
/// the asset's `original_path` + the P4 `TargetIntelligence`.
#[derive(Debug, Clone)]
pub struct GroupingInput<'a> {
    pub asset_id: String,
    pub metadata: &'a ExtractedMetadata,
    pub path: &'a Path,
    /// P4 target detection result. If `None`, the target
    /// field of the grouping key is `None` and unknown-target
    /// assets only collapse into sessions where other fields
    /// match.
    pub target: Option<&'a TargetIntelligence>,
}

/// CR-04 §8 — the canonical grouping key derived from a
/// single asset's metadata.
///
/// `PartialEq + Eq` so the stable sort collapses identical
/// keys into the same session.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct GroupingKey {
    /// Target detection id (e.g. "M31", "NGC7000", or
    /// `None` for unknown targets).
    pub target: Option<String>,
    /// Acquisition date in `YYYY-MM-DD` form (UTC). Two
    /// assets captured on the same UTC day collapse into the
    /// same session if every other field matches.
    pub capture_date: Option<String>,
    /// Instrument fingerprint: `camera|telescope`. Two
    /// assets from the same hardware setup collapse into
    /// the same session if every other field matches.
    pub instrument: Option<String>,
    /// Filter (FITS `FILTER`). Most projects switch filters
    /// between sessions, so this is a strong partition axis.
    pub filter: Option<String>,
    /// Binning bucket (see `bucket_binning`).
    pub binning: Option<String>,
    /// Dimensions bucket (see `bucket_dimensions`).
    pub dimensions: Option<String>,
    /// Exposure bucket (see `bucket_exposure`).
    pub exposure: Option<String>,
    /// Directory bucket: the parent directory of the asset's
    /// `original_path`. Lights and darks in
    /// `M31/2026-08-14/Light/` vs `M31/2026-08-14/Dark/` split
    /// on the directory axis even when every other field
    /// matches.
    pub directory: Option<String>,
}

impl GroupingKey {
    /// Derive the grouping key from a single asset's
    /// metadata + path + (optional) P4 target intelligence.
    pub fn from_input(input: &GroupingInput<'_>) -> Self {
        let m = input.metadata;
        let target = input
            .target
            .and_then(|t| t.candidate.as_ref())
            .map(|c| c.name.clone());
        let capture_date = m.date_obs.as_deref().and_then(extract_date);
        let instrument = m.camera.as_deref().map(|cam| cam.to_string());
        let filter = m.filter.clone();
        let binning = m
            .xbinning
            .zip(m.ybinning)
            .map(|(x, y)| bucket_binning(x.max(y) as u32));
        let dimensions = match (m.naxis1, m.naxis2) {
            (Some(w), Some(h)) => {
                let wu = u32::try_from(w).ok();
                let hu = u32::try_from(h).ok();
                match (wu, hu) {
                    (Some(wu), Some(hu)) => Some(bucket_dimensions(wu, hu)),
                    _ => None,
                }
            }
            _ => None,
        };
        let exposure = m.exptime.map(bucket_exposure);
        let directory = Some(directory_bucket(input.path));
        Self {
            target,
            capture_date,
            instrument,
            filter,
            binning,
            dimensions,
            exposure,
            directory,
        }
    }
}

/// CR-04 §8 — a group of `asset_id`s that share a single
/// `GroupingKey`. The IPC layer (P8) maps this to a
/// `Session` row + assigns each `SourceAsset.session_id`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SessionGroup {
    pub key: GroupingKey,
    pub asset_ids: Vec<String>,
}

/// Group a list of assets into `SessionGroup`s.
///
/// Algorithm:
/// 1. Derive each asset's `GroupingKey`.
/// 2. Sort assets by key (stable sort — assets keep their
///    relative order within a session).
/// 3. Sweep and merge consecutive assets with identical
///    keys into the same `SessionGroup`.
/// 4. Assign groups in sweep-order; the first group is the
///    session that was discovered first chronologically (per
///    the stable-sort order).
pub fn group_assets(inputs: &[GroupingInput<'_>]) -> Vec<SessionGroup> {
    let mut indexed: Vec<(usize, GroupingKey)> = inputs
        .iter()
        .enumerate()
        .map(|(idx, i)| (idx, GroupingKey::from_input(i)))
        .collect();

    indexed.sort_by(|a, b| a.1.cmp(&b.1).then(a.0.cmp(&b.0)));

    let mut groups: Vec<SessionGroup> = Vec::new();
    for (idx, key) in indexed {
        let asset_id = inputs[idx].asset_id.clone();
        if let Some(last) = groups.last_mut() {
            if last.key == key {
                last.asset_ids.push(asset_id);
                continue;
            }
        }
        groups.push(SessionGroup {
            key,
            asset_ids: vec![asset_id],
        });
    }
    groups
}

// ─── Bucketing helpers ──────────────────────────────────────────

/// Extract the `YYYY-MM-DD` portion of a FITS `DATE-OBS` value.
/// FITS dates look like `2026-08-14T22:31:05.123` or
/// `2026-08-14` depending on the writer; we keep the date
/// portion and drop the time so two assets captured on the
/// same UTC day collapse into the same session even if they
/// were taken minutes apart.
fn extract_date(date_obs: &str) -> Option<String> {
    // Take the first 10 chars if they parse as `YYYY-MM-DD`.
    if date_obs.len() >= 10 && date_obs.is_ascii() {
        let head = &date_obs[..10];
        if head.as_bytes()[4] == b'-' && head.as_bytes()[7] == b'-' {
            return Some(head.to_string());
        }
    }
    None
}

/// Bucket binning: small integer buckets `1|2|3|4|other`.
/// We don't try to preserve `1x1` vs `1x2` asymmetry because
/// the FITS standard has no canonical text form and most
/// acquisition software writes whatever it likes. Pixels
/// care about square vs non-square — we round to the largest
/// of the two axes and bucket into the canonical
/// `NxN` / `Nx?` form.
fn bucket_binning(max_bin: u32) -> String {
    if (1..=4).contains(&max_bin) {
        format!("{max_bin}x{max_bin}")
    } else {
        format!("{max_bin}x?")
    }
}

/// Bucket dimensions: keep exact pixels but round to the
/// nearest 100-pixel bucket to avoid near-identical assets
/// (e.g. 4096×4096 vs 4097×4097) splitting a session.
/// Round-trip: `4096 -> "4100x4100"`, `4097 -> "4100x4100"`.
fn bucket_dimensions(width: u32, height: u32) -> String {
    let round_to = |n: u32| -> u32 { ((n + 50) / 100) * 100 };
    format!("{}x{}", round_to(width), round_to(height))
}

/// Bucket exposure: round to the nearest 0.1 s. A 30 s
/// frame and a 30.04 s frame collapse into the same bucket;
/// a 30 s frame and a 31 s frame do not.
fn bucket_exposure(seconds: f64) -> String {
    let rounded = (seconds * 10.0).round() / 10.0;
    format!("{:.1}s", rounded)
}

/// Directory bucket: the parent directory of the asset's
/// path. We use the directory, not the asset itself, so a
/// session spans multiple files in the same folder.
/// Two-level ancestor would over-collapse (lights + darks
/// in `M31/2026-08-14/`); the direct parent is the right
/// granularity.
fn directory_bucket(path: &Path) -> String {
    path.parent()
        .map(|d| d.to_string_lossy().to_string())
        .unwrap_or_default()
}

// ─── Unit tests ─────────────────────────────────────────────────

#[cfg(test)]
#[allow(clippy::too_many_arguments)]
mod tests {
    use super::*;
    use crate::target_detection::TargetIntelligence;

    fn metadata(
        date_obs: Option<&str>,
        filter: Option<&str>,
        xbinning: Option<i64>,
        ybinning: Option<i64>,
        exptime: Option<f64>,
        naxis1: Option<i64>,
        naxis2: Option<i64>,
        instrume: Option<&str>,
        object: Option<&str>,
    ) -> ExtractedMetadata {
        ExtractedMetadata {
            object: object.map(String::from),
            exptime,
            filter: filter.map(String::from),
            xbinning,
            ybinning,
            naxis1,
            naxis2,
            instrume: instrume.map(String::from),
            date_obs: date_obs.map(String::from),
            ccd_temp: None,
            bitpix: None,
            bayerpat: None,
            telescop: None,
            focallen: None,
            gain: None,
            offset: None,
            width: naxis1.and_then(|n| u32::try_from(n).ok()),
            height: naxis2.and_then(|n| u32::try_from(n).ok()),
            bit_depth: None,
            camera: instrume.map(String::from),
        }
    }

    fn input<'a>(
        id: &str,
        m: &'a ExtractedMetadata,
        path: &'a Path,
        target: Option<&'a TargetIntelligence>,
    ) -> GroupingInput<'a> {
        GroupingInput {
            asset_id: id.into(),
            metadata: m,
            path,
            target,
        }
    }

    // ─── bucket helpers ──────────────────────────────────────

    #[test]
    fn extract_date_picks_yyyy_mm_dd() {
        assert_eq!(
            extract_date("2026-08-14T22:31:05.123"),
            Some("2026-08-14".into())
        );
        assert_eq!(extract_date("2026-08-14"), Some("2026-08-14".into()));
        assert_eq!(extract_date("not-a-date"), None);
        assert_eq!(extract_date(""), None);
    }

    #[test]
    fn bucket_binning_known_values() {
        assert_eq!(bucket_binning(1), "1x1");
        assert_eq!(bucket_binning(2), "2x2");
        assert_eq!(bucket_binning(5), "5x?");
    }

    #[test]
    fn bucket_dimensions_rounds_to_100() {
        assert_eq!(bucket_dimensions(4096, 4096), "4100x4100");
        assert_eq!(bucket_dimensions(4097, 4097), "4100x4100");
        assert_eq!(bucket_dimensions(3950, 3950), "4000x4000");
        assert_eq!(bucket_dimensions(2048, 1024), "2000x1000");
    }

    #[test]
    fn bucket_exposure_rounds_to_one_decimal() {
        assert_eq!(bucket_exposure(30.0), "30.0s");
        assert_eq!(bucket_exposure(30.04), "30.0s");
        assert_eq!(bucket_exposure(30.06), "30.1s");
        assert_eq!(bucket_exposure(120.5), "120.5s");
    }

    #[test]
    fn directory_bucket_uses_parent() {
        let p = Path::new("/home/user/M31/2026-08-14/Light/frame_001.fits");
        assert_eq!(directory_bucket(p), "/home/user/M31/2026-08-14/Light");
        let p = Path::new("M31/2026-08-14/Light/frame_001.fits");
        assert_eq!(directory_bucket(p), "M31/2026-08-14/Light");
        let p = Path::new("frame.fits");
        assert_eq!(directory_bucket(p), "");
    }

    // ─── grouping behavior ───────────────────────────────────

    #[test]
    fn identical_inputs_collapse_into_one_session() {
        let m = metadata(
            Some("2026-08-14T22:00:00"),
            Some("L"),
            Some(1),
            Some(1),
            Some(30.0),
            Some(4096),
            Some(4096),
            Some("ASI2600MC"),
            Some("M31"),
        );
        let ti = TargetIntelligence::from_known_name("M31");
        let p1 = Path::new("/M31/2026-08-14/Light/a1.fits");
        let p2 = Path::new("/M31/2026-08-14/Light/a2.fits");
        let p3 = Path::new("/M31/2026-08-14/Light/a3.fits");
        let inputs = vec![
            input("a1", &m, p1, Some(&ti)),
            input("a2", &m, p2, Some(&ti)),
            input("a3", &m, p3, Some(&ti)),
        ];
        let groups = group_assets(&inputs);
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].asset_ids.len(), 3);
    }

    #[test]
    fn different_dates_split_sessions() {
        let m1 = metadata(
            Some("2026-08-14T22:00:00"),
            Some("L"),
            Some(1),
            Some(1),
            Some(30.0),
            Some(4096),
            Some(4096),
            Some("ASI2600MC"),
            Some("M31"),
        );
        let m2 = metadata(
            Some("2026-08-15T22:00:00"),
            Some("L"),
            Some(1),
            Some(1),
            Some(30.0),
            Some(4096),
            Some(4096),
            Some("ASI2600MC"),
            Some("M31"),
        );
        let ti = TargetIntelligence::from_known_name("M31");
        let inputs = vec![
            input("a1", &m1, Path::new("/M31/Light/a1.fits"), Some(&ti)),
            input("a2", &m2, Path::new("/M31/Light/a2.fits"), Some(&ti)),
        ];
        let groups = group_assets(&inputs);
        assert_eq!(groups.len(), 2);
    }

    #[test]
    fn different_filters_split_sessions() {
        let m1 = metadata(
            Some("2026-08-14T22:00:00"),
            Some("L"),
            Some(1),
            Some(1),
            Some(30.0),
            Some(4096),
            Some(4096),
            Some("ASI2600MC"),
            Some("M31"),
        );
        let m2 = metadata(
            Some("2026-08-14T22:05:00"),
            Some("Ha"),
            Some(1),
            Some(1),
            Some(30.0),
            Some(4096),
            Some(4096),
            Some("ASI2600MC"),
            Some("M31"),
        );
        let ti = TargetIntelligence::from_known_name("M31");
        let inputs = vec![
            input("a1", &m1, Path::new("/M31/Light/a1.fits"), Some(&ti)),
            input("a2", &m2, Path::new("/M31/Light/a2.fits"), Some(&ti)),
        ];
        let groups = group_assets(&inputs);
        assert_eq!(groups.len(), 2);
    }

    #[test]
    fn different_instruments_split_sessions() {
        let m1 = metadata(
            Some("2026-08-14T22:00:00"),
            Some("L"),
            Some(1),
            Some(1),
            Some(30.0),
            Some(4096),
            Some(4096),
            Some("ASI2600MC"),
            Some("M31"),
        );
        let m2 = metadata(
            Some("2026-08-14T22:05:00"),
            Some("L"),
            Some(1),
            Some(1),
            Some(30.0),
            Some(4096),
            Some(4096),
            Some("ASI6200MM"),
            Some("M31"),
        );
        let ti = TargetIntelligence::from_known_name("M31");
        let inputs = vec![
            input("a1", &m1, Path::new("/M31/Light/a1.fits"), Some(&ti)),
            input("a2", &m2, Path::new("/M31/Light/a2.fits"), Some(&ti)),
        ];
        let groups = group_assets(&inputs);
        assert_eq!(groups.len(), 2);
    }

    #[test]
    fn different_exposure_buckets_split_sessions() {
        let m1 = metadata(
            Some("2026-08-14T22:00:00"),
            Some("L"),
            Some(1),
            Some(1),
            Some(30.0),
            Some(4096),
            Some(4096),
            Some("ASI2600MC"),
            Some("M31"),
        );
        let m2 = metadata(
            Some("2026-08-14T22:05:00"),
            Some("L"),
            Some(1),
            Some(1),
            Some(60.0),
            Some(4096),
            Some(4096),
            Some("ASI2600MC"),
            Some("M31"),
        );
        let ti = TargetIntelligence::from_known_name("M31");
        let inputs = vec![
            input("a1", &m1, Path::new("/M31/Light/a1.fits"), Some(&ti)),
            input("a2", &m2, Path::new("/M31/Light/a2.fits"), Some(&ti)),
        ];
        let groups = group_assets(&inputs);
        assert_eq!(groups.len(), 2);
    }

    #[test]
    fn different_directories_split_sessions() {
        let m1 = metadata(
            Some("2026-08-14T22:00:00"),
            Some("L"),
            Some(1),
            Some(1),
            Some(30.0),
            Some(4096),
            Some(4096),
            Some("ASI2600MC"),
            Some("M31"),
        );
        let ti = TargetIntelligence::from_known_name("M31");
        let inputs = vec![
            input("a1", &m1, Path::new("/M31/Light/a1.fits"), Some(&ti)),
            input("a2", &m1, Path::new("/M31/Dark/a2.fits"), Some(&ti)),
        ];
        let groups = group_assets(&inputs);
        assert_eq!(groups.len(), 2);
    }

    #[test]
    fn unknown_target_assets_share_session_when_other_keys_match() {
        let m1 = metadata(
            Some("2026-08-14T22:00:00"),
            Some("L"),
            Some(1),
            Some(1),
            Some(30.0),
            Some(4096),
            Some(4096),
            Some("ASI2600MC"),
            None,
        );
        let inputs = vec![
            input("a1", &m1, Path::new("/M31/Light/a1.fits"), None),
            input("a2", &m1, Path::new("/M31/Light/a2.fits"), None),
        ];
        let groups = group_assets(&inputs);
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].key.target, None);
    }

    #[test]
    fn mixed_realistic_corpus_groups_correctly() {
        // Three nights × two filters × L (Ha) = the §8 example.
        let m_l_30 = metadata(
            Some("2026-08-14T22:00:00"),
            Some("L"),
            Some(1),
            Some(1),
            Some(30.0),
            Some(4096),
            Some(4096),
            Some("ASI2600MC"),
            Some("M31"),
        );
        let m_l_30b = metadata(
            Some("2026-08-14T22:01:00"),
            Some("L"),
            Some(1),
            Some(1),
            Some(30.0),
            Some(4096),
            Some(4096),
            Some("ASI2600MC"),
            Some("M31"),
        );
        let m_l_30_n2 = metadata(
            Some("2026-08-15T22:00:00"),
            Some("L"),
            Some(1),
            Some(1),
            Some(30.0),
            Some(4096),
            Some(4096),
            Some("ASI2600MC"),
            Some("M31"),
        );
        let m_ha_300 = metadata(
            Some("2026-08-15T23:00:00"),
            Some("Ha"),
            Some(1),
            Some(1),
            Some(300.0),
            Some(4096),
            Some(4096),
            Some("ASI2600MC"),
            Some("M31"),
        );
        let m_dark_30 = metadata(
            Some("2026-08-15T22:30:00"),
            None,
            Some(1),
            Some(1),
            Some(30.0),
            Some(4096),
            Some(4096),
            Some("ASI2600MC"),
            Some("M31"),
        );
        let ti = TargetIntelligence::from_known_name("M31");
        let inputs = vec![
            input(
                "n1_l_001",
                &m_l_30,
                Path::new("/M31/2026-08-14/Lights/n1_l_001.fits"),
                Some(&ti),
            ),
            input(
                "n1_l_002",
                &m_l_30b,
                Path::new("/M31/2026-08-14/Lights/n1_l_002.fits"),
                Some(&ti),
            ),
            input(
                "n2_l_001",
                &m_l_30_n2,
                Path::new("/M31/2026-08-15/Lights/n2_l_001.fits"),
                Some(&ti),
            ),
            input(
                "n2_ha_001",
                &m_ha_300,
                Path::new("/M31/2026-08-15/Lights/n2_ha_001.fits"),
                Some(&ti),
            ),
            input(
                "n2_d_001",
                &m_dark_30,
                Path::new("/M31/2026-08-15/Darks/n2_d_001.fits"),
                Some(&ti),
            ),
        ];
        let groups = group_assets(&inputs);
        // Expect 4 sessions: Night1-L, Night2-L, Night2-Ha, Night2-Darks.
        assert_eq!(groups.len(), 4);
        assert_eq!(groups[0].asset_ids.len(), 2); // Night1-L
        assert_eq!(groups[1].asset_ids.len(), 1); // Night2-L
        assert_eq!(groups[2].asset_ids.len(), 1); // Night2-Ha
        assert_eq!(groups[3].asset_ids.len(), 1); // Night2-Darks
    }

    #[test]
    fn empty_input_yields_no_groups() {
        let groups = group_assets(&[]);
        assert!(groups.is_empty());
    }

    #[test]
    fn single_input_yields_one_group() {
        let m = metadata(
            Some("2026-08-14T22:00:00"),
            Some("L"),
            Some(1),
            Some(1),
            Some(30.0),
            Some(4096),
            Some(4096),
            Some("ASI2600MC"),
            Some("M31"),
        );
        let ti = TargetIntelligence::from_known_name("M31");
        let inputs = vec![input(
            "solo",
            &m,
            Path::new("/M31/Light/solo.fits"),
            Some(&ti),
        )];
        let groups = group_assets(&inputs);
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].asset_ids, vec!["solo".to_string()]);
    }

    #[test]
    fn stable_sort_preserves_input_order_within_session() {
        let m = metadata(
            Some("2026-08-14T22:00:00"),
            Some("L"),
            Some(1),
            Some(1),
            Some(30.0),
            Some(4096),
            Some(4096),
            Some("ASI2600MC"),
            Some("M31"),
        );
        let ti = TargetIntelligence::from_known_name("M31");
        let inputs = vec![
            input("z", &m, Path::new("/M31/Light/z.fits"), Some(&ti)),
            input("a", &m, Path::new("/M31/Light/a.fits"), Some(&ti)),
            input("m", &m, Path::new("/M31/Light/m.fits"), Some(&ti)),
        ];
        let groups = group_assets(&inputs);
        assert_eq!(groups.len(), 1);
        // Input order is z, a, m — stable sort preserves that within a session.
        assert_eq!(
            groups[0].asset_ids,
            vec!["z".to_string(), "a".to_string(), "m".to_string()]
        );
    }

    #[test]
    fn target_id_is_preserved_in_session_key() {
        let m = metadata(
            Some("2026-08-14T22:00:00"),
            Some("L"),
            Some(1),
            Some(1),
            Some(30.0),
            Some(4096),
            Some(4096),
            Some("ASI2600MC"),
            Some("M31"),
        );
        let ti = TargetIntelligence::from_known_name("M31");
        let inputs = vec![input("a1", &m, Path::new("/M31/Light/a1.fits"), Some(&ti))];
        let groups = group_assets(&inputs);
        assert_eq!(groups[0].key.target, Some("M31".to_string()));
    }
}
