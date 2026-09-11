//! CR-04 §9 — Target detection.
//!
//! Identifies the astronomical target the user captured, using
//! the evidence hierarchy from CR-04 §9:
//!
//! Highest confidence:
//! 1. Explicit FITS OBJECT keyword (ExtractedMetadata.object).
//! 2. Explicit user metadata (out-of-band; deferred).
//! 3. Recognizable acquisition metadata.
//! 4. Directory/project naming.
//! 5. Filename patterns.
//!
//! Lower confidence (deferred to a future CR):
//! 6. Plate solving — image-based; needs astrometry.net or local
//!    solver; not in scope for P4.
//! 7. Coordinate inference — depends on plate solving.
//! 8. Image-based recognition — depends on astroforge-ai.
//!
//! Each inference carries:
//! - the `TargetCandidate` (name + aliases)
//! - a confidence score 0.0..=1.0
//! - a list of `TargetObservation`s so the UI can show
//!   "Observation → Evidence → Confidence → Decision" (per
//!   CR-04 §2.3 ADR-04.2)
//! - the `TargetKind` (Galaxy / Nebula / Cluster / Star / Planet /
//!   Moon / Comet / Unknown) for downstream capture-analysis
//!   (P6) and narrowband detection (P7).

use crate::import_scan::ExtractedMetadata;
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::OnceLock;

/// CR-04 §9 — the astronomical kind of the detected target.
///
/// Drives the downstream capture-analysis routing (P6) and
/// narrowband detection (P7). Conservative enum: future CRs can
/// add kinds without breaking consumers because the
/// serde rename_all = "snake_case" pattern keeps the wire format
/// stable.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TargetKind {
    Galaxy,
    Nebula,
    Cluster,
    Star,
    Planet,
    Moon,
    Comet,
    Unknown,
}

impl TargetKind {
    pub fn as_str(self) -> &'static str {
        match self {
            TargetKind::Galaxy => "galaxy",
            TargetKind::Nebula => "nebula",
            TargetKind::Cluster => "cluster",
            TargetKind::Star => "star",
            TargetKind::Planet => "planet",
            TargetKind::Moon => "moon",
            TargetKind::Comet => "comet",
            TargetKind::Unknown => "unknown",
        }
    }
}

/// CR-04 §9 — one candidate for the captured target.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TargetCandidate {
    pub name: String,
    pub aliases: Vec<String>,
    pub kind: TargetKind,
}

/// One piece of evidence that contributed to a target detection.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TargetObservation {
    pub signal: String,
    pub value: String,
    pub weight: f64,
    pub confidence: f64,
}

impl TargetObservation {
    pub fn new(
        signal: impl Into<String>,
        value: impl Into<String>,
        weight: f64,
        confidence: f64,
    ) -> Self {
        Self {
            signal: signal.into(),
            value: value.into(),
            weight,
            confidence,
        }
    }
}

/// CR-04 §9 — the result of target detection for a single dataset.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TargetIntelligence {
    pub candidate: Option<TargetCandidate>,
    pub confidence: f64,
    pub observations: Vec<TargetObservation>,
}

impl TargetIntelligence {
    /// Build a `TargetIntelligence` for a known canonical target
    /// name (e.g. "M31"). Looks up the candidate in the embedded
    /// catalog and emits a single high-confidence observation.
    ///
    /// Useful for tests, fixtures, and the IPC layer (P8) when
    /// the caller has already resolved the target outside the
    /// FITS-metadata path.
    pub fn from_known_name(name: &str) -> Self {
        match lookup(name) {
            Some(c) => Self {
                candidate: Some(c.clone()),
                confidence: 1.0,
                observations: vec![TargetObservation::new("explicit_lookup", name, 1.0, 1.0)],
            },
            None => Self {
                candidate: None,
                confidence: 0.0,
                observations: vec![],
            },
        }
    }
}

/// Normalize an arbitrary target-name string for matching.
///
/// Strips whitespace, uppercases, removes punctuation that
/// commonly varies between catalogs (spaces, dots, dashes,
/// underscores). The intent is "M31" and "M 31" and "m-31"
/// all match the same canonical form.
pub fn normalize(name: &str) -> String {
    let mut out = String::with_capacity(name.len());
    for ch in name.chars() {
        if ch.is_alphanumeric() {
            out.extend(ch.to_uppercase());
        }
    }
    out
}

/// Look up a target by its normalized form (returns the first
/// match, prioritizing the canonical name over aliases).
pub fn lookup(name: &str) -> Option<&'static TargetCandidate> {
    let normalized = normalize(name);
    known_targets()
        .iter()
        .find(|t| normalize(&t.name) == normalized)
        .or_else(|| {
            known_targets()
                .iter()
                .find(|t| t.aliases.iter().any(|a| normalize(a) == normalized))
        })
}

/// A small embedded catalog of well-known targets.
///
/// Covers the bright Messier objects that smart-telescope users
/// most commonly capture (M31 / M42 / M51 / M101 / NGC 7000 /
/// IC 1396 / Sh2-155), plus the planets and the Moon. This is
/// intentionally conservative — it powers filename/directory
/// heuristics, not the authoritative catalog.
///
/// Real catalog matching against the full NGC/IC/Sh2 lists
/// belongs to a future CR (P10 / AI hook).
pub fn known_targets() -> &'static [TargetCandidate] {
    static CATALOG: OnceLock<Vec<TargetCandidate>> = OnceLock::new();
    CATALOG.get_or_init(init_catalog)
}

fn init_catalog() -> Vec<TargetCandidate> {
    vec![
        // Galaxies
        target("M31", &["ANDROMEDA", "NGC 224"], TargetKind::Galaxy),
        target("M33", &["TRIANGULUM", "NGC 598"], TargetKind::Galaxy),
        target("M51", &["WHIRLPOOL", "NGC 5194"], TargetKind::Galaxy),
        target("M81", &["BODE", "NGC 3031"], TargetKind::Galaxy),
        target("M82", &["CIGAR", "NGC 3034"], TargetKind::Galaxy),
        target("M101", &["PINWHEEL", "NGC 5457"], TargetKind::Galaxy),
        target("M104", &["SOMBRERO", "NGC 4594"], TargetKind::Galaxy),
        target("M106", &["NGC 4258"], TargetKind::Galaxy),
        target("NGC 253", &[], TargetKind::Galaxy),
        target("NGC 891", &[], TargetKind::Galaxy),
        // Nebulae
        target("M42", &["ORION", "NGC 1976"], TargetKind::Nebula),
        target("M8", &["LAGOON", "NGC 6523"], TargetKind::Nebula),
        target("M16", &["EAGLE", "NGC 6611"], TargetKind::Nebula),
        target("M17", &["OMEGA", "NGC 6618", "SWAN"], TargetKind::Nebula),
        target("M20", &["TRIFID", "NGC 6514"], TargetKind::Nebula),
        target("M27", &["DUMBBELL", "NGC 6853"], TargetKind::Nebula),
        target("M57", &["RING", "NGC 6720"], TargetKind::Nebula),
        target("M76", &["LITTLE DUMBBELL", "NGC 650"], TargetKind::Nebula),
        target("M97", &["OWL", "NGC 3587"], TargetKind::Nebula),
        target("NGC 7000", &["NORTH AMERICA"], TargetKind::Nebula),
        target("IC 1396", &["ELEPHANT TRUNK"], TargetKind::Nebula),
        target("IC 5070", &["PELICAN"], TargetKind::Nebula),
        target("NGC 6960", &["VEIL", "WESTERN VEIL"], TargetKind::Nebula),
        target("NGC 6992", &["VEIL", "EASTERN VEIL"], TargetKind::Nebula),
        target("Sh2-155", &["CAVE"], TargetKind::Nebula),
        target("Sh2-101", &["TULIP"], TargetKind::Nebula),
        target("B33", &["HORSEHEAD"], TargetKind::Nebula),
        target("NGC 281", &["PACMAN"], TargetKind::Nebula),
        // Clusters
        target("M13", &["HERCULES", "NGC 6205"], TargetKind::Cluster),
        target("M22", &["NGC 6656"], TargetKind::Cluster),
        target("M45", &["PLEIADES", "SEVEN SISTERS"], TargetKind::Cluster),
        target("NGC 869", &["DOUBLE CLUSTER"], TargetKind::Cluster),
        target("NGC 884", &["DOUBLE CLUSTER"], TargetKind::Cluster),
        // Planets
        target("JUPITER", &[], TargetKind::Planet),
        target("SATURN", &[], TargetKind::Planet),
        target("MARS", &[], TargetKind::Planet),
        target("VENUS", &[], TargetKind::Planet),
        target("MERCURY", &[], TargetKind::Planet),
        target("URANUS", &[], TargetKind::Planet),
        target("NEPTUNE", &[], TargetKind::Planet),
        // Moon
        target("MOON", &["LUNA"], TargetKind::Moon),
    ]
}

// Helper for building catalog rows at runtime in init_catalog.
fn target(name: &str, aliases: &[&str], kind: TargetKind) -> TargetCandidate {
    TargetCandidate {
        name: name.to_string(),
        aliases: aliases.iter().map(|s| s.to_string()).collect(),
        kind,
    }
}

/// CR-04 §9 — run target detection on a dataset's metadata + path.
///
/// Inputs:
/// - `metadata`: the `ExtractedMetadata` from `import_scan`
///   (P1). Carries the FITS OBJECT keyword if present.
/// - `path`: the source path; used for filename + directory
///   pattern matching.
///
/// Output:
/// - A typed `TargetIntelligence` describing the candidate, the
///   confidence, and the evidence list.
///
/// Evidence priority (highest first):
/// 1. FITS OBJECT keyword (weight 1.0, confidence 1.0).
/// 2. Filename pattern (weight 0.6–0.8, confidence ~0.7).
/// 3. Directory pattern (weight 0.5, confidence ~0.7).
pub fn detect(metadata: &ExtractedMetadata, path: &Path) -> TargetIntelligence {
    let mut observations: Vec<TargetObservation> = Vec::new();

    // 1. FITS OBJECT keyword — highest priority.
    if let Some(object) = metadata.object.as_deref() {
        observations.push(TargetObservation::new("fits_object", object, 1.0, 1.0));
        if let Some(candidate) = lookup(object) {
            return TargetIntelligence {
                candidate: Some(candidate.clone()),
                confidence: 1.0,
                observations,
            };
        }
        // OBJECT keyword present but not in our small catalog.
        // Still report it as a candidate (Unknown kind) so the
        // UI can show the user what the metadata says.
        return TargetIntelligence {
            candidate: Some(TargetCandidate {
                name: object.to_string(),
                aliases: vec![],
                kind: TargetKind::Unknown,
            }),
            confidence: 0.9,
            observations,
        };
    }

    // 2. Filename pattern (e.g. "m31_ha_001.fits", "M42_LIGHT_001.fits").
    if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
        if let Some(candidate) = match_filename(name) {
            observations.push(TargetObservation::new("filename_pattern", name, 0.7, 0.7));
            return TargetIntelligence {
                candidate: Some(candidate),
                confidence: 0.7,
                observations,
            };
        } else {
            // Record that we looked but didn't find — useful for
            // debugging and for the UI's "no signal" prompt.
            observations.push(TargetObservation::new(
                "filename_pattern",
                format!("no_match:{}", name),
                0.1,
                0.1,
            ));
        }
    }

    // 3. Directory pattern (e.g. "/M31/Light/", "/NGC_7000/Ha/").
    //    Walk every ancestor directory so nested paths still find
    //    the target name (e.g. /data/NGC_7000/Light/img.fits).
    if let Some((candidate, dir_name)) = match_any_ancestor(path) {
        observations.push(TargetObservation::new(
            "directory_pattern",
            dir_name,
            0.5,
            0.7,
        ));
        return TargetIntelligence {
            candidate: Some(candidate),
            confidence: 0.6,
            observations,
        };
    } else {
        // Record that we looked but didn't find. Try the parent
        // dir name for the observation log so the UI can show
        // which directory was checked.
        let dir_name = path
            .parent()
            .and_then(|p| p.file_name())
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_string();
        observations.push(TargetObservation::new(
            "directory_pattern",
            format!("no_match:{}", dir_name),
            0.1,
            0.1,
        ));
    }

    // 4. No signal at all. The UI should ask the user.
    observations.push(TargetObservation::new(
        "no_signal",
        "no_object_no_filename_no_directory",
        0.1,
        0.1,
    ));
    TargetIntelligence {
        candidate: None,
        confidence: 0.0,
        observations,
    }
}

/// Match a candidate name within an arbitrary string (filename,
/// directory, project name). The string is split on non-alphanumeric
/// boundaries and each token is normalized + looked up.
fn match_filename(s: &str) -> Option<TargetCandidate> {
    let normalized = normalize(s);
    // Try the full string first (handles "M31_ha_001" because
    // the underscore is stripped).
    if let Some(candidate) = lookup(&normalized) {
        return Some(candidate.clone());
    }
    // Try each token (handles "M31 30x300s" by picking "M31").
    for token in s.split(|c: char| !c.is_alphanumeric()) {
        if token.is_empty() {
            continue;
        }
        if let Some(candidate) = lookup(token) {
            return Some(candidate.clone());
        }
    }
    None
}

/// Walk every ancestor directory of `path` and return the first
/// ancestor whose name matches a known target. Used by
/// `detect()` so a path like `/data/NGC_7000/Light/img.fits`
/// finds "NGC 7000" two levels up rather than just "Light".
fn match_any_ancestor(path: &Path) -> Option<(TargetCandidate, String)> {
    let mut current = path.parent();
    while let Some(dir) = current {
        let name = dir.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if let Some(candidate) = match_filename(name) {
            return Some((candidate, name.to_string()));
        }
        current = dir.parent();
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_strips_punctuation_and_uppercases() {
        assert_eq!(normalize("M 31"), "M31");
        assert_eq!(normalize("m-31"), "M31");
        assert_eq!(normalize("NGC_7000"), "NGC7000");
        assert_eq!(normalize("Andromeda"), "ANDROMEDA");
    }

    #[test]
    fn lookup_finds_canonical_name() {
        let candidate = lookup("M31").unwrap();
        assert_eq!(candidate.name, "M31");
        assert_eq!(candidate.kind, TargetKind::Galaxy);
    }

    #[test]
    fn lookup_finds_alias() {
        let candidate = lookup("Andromeda").unwrap();
        assert_eq!(candidate.name, "M31");
    }

    #[test]
    fn lookup_returns_none_for_unknown() {
        assert!(lookup("foobar").is_none());
    }

    #[test]
    fn detect_fits_object_keyword_is_highest_priority() {
        let md = ExtractedMetadata {
            object: Some("M42".into()),
            ..Default::default()
        };
        let path = Path::new("/random/path/img_001.fits");
        let intel = detect(&md, path);
        assert_eq!(intel.candidate.unwrap().name, "M42");
        assert_eq!(intel.confidence, 1.0);
    }

    #[test]
    fn detect_fits_object_unknown_target_returns_unknown_kind() {
        let md = ExtractedMetadata {
            object: Some("Foobar".into()),
            ..Default::default()
        };
        let path = Path::new("/random/path/img_001.fits");
        let intel = detect(&md, path);
        assert_eq!(intel.candidate.as_ref().unwrap().name, "Foobar");
        assert_eq!(intel.candidate.as_ref().unwrap().kind, TargetKind::Unknown);
        assert!(intel.confidence >= 0.8);
    }

    #[test]
    fn detect_filename_pattern_finds_target() {
        let md = ExtractedMetadata::default();
        let path = Path::new("/data/m31_ha_001.fits");
        let intel = detect(&md, path);
        assert_eq!(intel.candidate.unwrap().name, "M31");
        assert!(intel.confidence >= 0.5);
    }

    #[test]
    fn detect_directory_pattern_finds_target() {
        let md = ExtractedMetadata::default();
        let path = Path::new("/data/NGC_7000/Light/img_001.fits");
        let intel = detect(&md, path);
        assert_eq!(intel.candidate.unwrap().name, "NGC 7000");
        assert!(intel.confidence >= 0.5);
    }

    #[test]
    fn detect_filename_alias_finds_target() {
        let md = ExtractedMetadata::default();
        let path = Path::new("/data/andromeda_session1/img_001.fits");
        let intel = detect(&md, path);
        assert_eq!(intel.candidate.unwrap().name, "M31");
    }

    #[test]
    fn detect_planet_keyword_is_a_planet() {
        let md = ExtractedMetadata {
            object: Some("Jupiter".into()),
            ..Default::default()
        };
        let path = Path::new("/data/jupiter/img_001.fits");
        let intel = detect(&md, path);
        assert_eq!(intel.candidate.unwrap().kind, TargetKind::Planet);
    }

    #[test]
    fn detect_no_signal_returns_no_candidate() {
        let md = ExtractedMetadata::default();
        let path = Path::new("/data/foo/bar.fits");
        let intel = detect(&md, path);
        assert!(intel.candidate.is_none());
        assert_eq!(intel.confidence, 0.0);
    }

    #[test]
    fn observations_are_recorded_for_audit() {
        let md = ExtractedMetadata {
            object: Some("M42".into()),
            ..Default::default()
        };
        let path = Path::new("/data/img_001.fits");
        let intel = detect(&md, path);
        let signals: Vec<&str> = intel
            .observations
            .iter()
            .map(|o| o.signal.as_str())
            .collect();
        assert!(signals.contains(&"fits_object"));
    }

    #[test]
    fn known_targets_catalog_has_well_known_objects() {
        let names: Vec<String> = known_targets().iter().map(|t| t.name.clone()).collect();
        // Catalog stores canonical (often uppercased) forms;
        // check both case-sensitive and case-insensitive forms.
        assert!(names.iter().any(|n| n == "M31" || n == "m31"));
        assert!(names.iter().any(|n| n == "M42" || n == "m42"));
        assert!(names.iter().any(|n| n == "NGC 7000" || n == "ngc 7000"));
        assert!(names.iter().any(|n| n.to_uppercase() == "JUPITER"));
        assert!(names.iter().any(|n| n.to_uppercase() == "MOON"));
    }
}
