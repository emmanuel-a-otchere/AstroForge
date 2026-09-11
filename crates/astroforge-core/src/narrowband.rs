//! CR-04 P7 — Narrowband Detection (CR-04 §11).
//!
//! Per CR-04 §11, AstroForge should identify likely
//! narrowband acquisition. Signals include:
//!
//! - FILTER metadata (FITS `FILTER`);
//! - filenames (e.g. `NGC7000_HA_001.fits`);
//! - channel-specific acquisition groups.
//!
//! The module normalises filter strings into a
//! `NarrowbandChannel` enum, groups assets by channel, and
//! suggests a composition (HOO / SHO / LRGB / Mono).
//!
//! **No silent composition.** The §11 critical rule (same
//! shape as §10): "The system should recommend, not
//! silently impose, the final composition." The IPC layer
//! (P8) and UI (P9) must surface the suggestion to the user
//! before any processing strategy is committed.
//!
//! ## Composes with P3 + P4 + P5 + P6
//!
//! - P3 `classification::FrameKind` filters out
//!   calibration frames; the IPC layer (P8) passes only
//!   Light frames to `detect_narrowband`.
//! - P5 `session_grouping` groups assets into sessions;
//!   narrowband detection runs per-session.
//! - P6 `capture_analysis` handles deep-sky vs planetary
//!   routing; narrowband detection is orthogonal to that
//!   axis and runs after P6.
//!
//! ## No drift on missing metadata
//!
//! Missing filter metadata → `Unknown` channel. Two
//! assets only collapse into the same channel if a *known*
//! filter matches. Filename heuristics contribute as
//! secondary signals (lower confidence) when filter
//! metadata is missing.

use crate::import_scan::ExtractedMetadata;
use serde::{Deserialize, Serialize};

/// CR-04 §11 — the canonical narrowband channels.
///
/// `Unknown` is a first-class variant: assets with
/// unrecognised filter strings fall into `Unknown` rather
/// than being silently dropped. The UI must surface
/// unknown-channel assets so the user can correct the
/// filter mapping.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NarrowbandChannel {
    /// Hydrogen-alpha (656.28 nm). Includes Hα, Ha, HA,
    /// HALPHA, H-ALPHA, H_ALPHA variants.
    #[serde(rename = "ha")]
    Ha,
    /// Oxygen III (500.7 nm). Includes OIII, O3, OIII,
    /// OIII variants.
    #[serde(rename = "oiii")]
    OIII,
    /// Sulphur II (671.6 / 672.4 nm). Includes SII, S2,
    /// SULPHUR, S-II, S_II variants.
    #[serde(rename = "sii")]
    SII,
    /// Luminance broadband. Includes L, LUM, LUMINANCE.
    #[serde(rename = "l")]
    L,
    /// Red broadband. Includes R, RED.
    #[serde(rename = "r")]
    R,
    /// Green broadband. Includes G, GREEN.
    #[serde(rename = "g")]
    G,
    /// Blue broadband. Includes B, BLUE.
    #[serde(rename = "b")]
    B,
    /// Continuum / clear / empty filter. Includes CLEAR,
    /// UV, IR, IR-CUT.
    #[serde(rename = "continuum")]
    Continuum,
    /// Unrecognised filter string. Assets fall here when
    /// the filter metadata is present but doesn't match any
    /// known channel.
    #[serde(rename = "unknown")]
    Unknown,
}

impl NarrowbandChannel {
    pub fn as_str(self) -> &'static str {
        match self {
            NarrowbandChannel::Ha => "ha",
            NarrowbandChannel::OIII => "oiii",
            NarrowbandChannel::SII => "sii",
            NarrowbandChannel::L => "l",
            NarrowbandChannel::R => "r",
            NarrowbandChannel::G => "g",
            NarrowbandChannel::B => "b",
            NarrowbandChannel::Continuum => "continuum",
            NarrowbandChannel::Unknown => "unknown",
        }
    }
}

/// CR-04 §11 — a group of asset ids that share a single
/// `NarrowbandChannel`. The IPC layer (P8) maps this to a
/// session's channel-grouped subsets.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChannelGroup {
    pub channel: NarrowbandChannel,
    pub asset_ids: Vec<String>,
}

/// CR-04 §11 — the composition suggestion. Mirrors the
/// §11 example ("Ha 124 frames, OIII 108 frames → HOO")
/// and the classic narrowband palettes:
///
/// - **Mono** — single channel present (or only `L` /
///   continuum / unknown).
/// - **HOO** — Hubble-style bi-colour: Ha as red, OIII as
///   blue + green.
/// - **SHO** — Hubble-style tri-colour: SII as red, Ha as
///   green, OIII as blue.
/// - **LRGB** — broadband luminance + RGB colour.
/// - **HOO + SHO** — both palettes possible; the UI
///   surfaces both.
/// - **None** — no recognisable narrowband structure.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NarrowbandComposition {
    None,
    Mono,
    Hoo,
    Sho,
    Lrgb,
    /// Both HOO and SHO possible (all three narrowband
    /// channels present).
    HooOrSho,
}

impl NarrowbandComposition {
    pub fn as_str(self) -> &'static str {
        match self {
            NarrowbandComposition::None => "none",
            NarrowbandComposition::Mono => "mono",
            NarrowbandComposition::Hoo => "hoo",
            NarrowbandComposition::Sho => "sho",
            NarrowbandComposition::Lrgb => "lrgb",
            NarrowbandComposition::HooOrSho => "hoo_or_sho",
        }
    }
}

/// CR-04 §11 — one observation that contributed to
/// narrowband detection. Mirrors the
/// `FrameObservation` (P3) + `TargetObservation` (P4) +
/// `CaptureObservation` (P6) shape.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NarrowbandObservation {
    /// Signal id (e.g. "filter_metadata",
    /// "filename_pattern").
    pub signal: String,
    /// Observed value (e.g. "Ha", "OIII",
    /// "ngc7000_ha_001.fits").
    pub value: String,
    /// 0.0..=1.0. Per-signal weight.
    pub weight: f64,
    /// 0.0..=1.0. Per-signal confidence.
    pub confidence: f64,
    /// Channel this observation resolves to (if any).
    pub resolves_to: Option<NarrowbandChannel>,
}

/// CR-04 §11 — the result of narrowband detection for one
/// session.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NarrowbandAnalysis {
    /// Per-channel asset groups. Always populated; empty
    /// groups for missing channels are omitted.
    pub channels: Vec<ChannelGroup>,
    pub composition_suggestion: NarrowbandComposition,
    /// 0.0..=1.0. Confidence in the suggestion. Below ~0.5
    /// the UI should flag the inference (per CR-04 §13
    /// ambiguity UX).
    pub confidence: f64,
    pub observations: Vec<NarrowbandObservation>,
}

/// CR-04 §11 — the input to narrowband detection for a
/// single asset. The IPC layer (P8) constructs this from
/// `ExtractedMetadata` + the asset's `asset_id` +
/// `original_path`.
#[derive(Debug, Clone)]
pub struct AssetRef<'a> {
    pub asset_id: String,
    pub metadata: &'a ExtractedMetadata,
    pub path: &'a str,
}

/// Detect narrowband acquisition in a session's assets.
///
/// Pure function: no I/O, no DB, no time. Returns a
/// `NarrowbandAnalysis` with per-channel groups +
/// composition suggestion + evidence observations.
pub fn detect_narrowband(assets: &[AssetRef<'_>]) -> NarrowbandAnalysis {
    if assets.is_empty() {
        return NarrowbandAnalysis {
            channels: Vec::new(),
            composition_suggestion: NarrowbandComposition::None,
            confidence: 0.0,
            observations: Vec::new(),
        };
    }

    let mut observations: Vec<NarrowbandObservation> = Vec::new();
    let mut channel_buckets: std::collections::BTreeMap<NarrowbandChannel, Vec<String>> =
        std::collections::BTreeMap::new();

    for asset in assets {
        let (channel, obs) = resolve_channel(asset);
        if let Some(c) = channel {
            observations.extend(obs);
            channel_buckets
                .entry(c)
                .or_default()
                .push(asset.asset_id.clone());
        } else {
            // Unknown-channel assets still get observed for
            // the UI, but don't contribute to any channel
            // bucket.
            observations.extend(obs);
            channel_buckets
                .entry(NarrowbandChannel::Unknown)
                .or_default()
                .push(asset.asset_id.clone());
        }
    }

    let present: Vec<NarrowbandChannel> = channel_buckets
        .iter()
        .filter(|(c, ids)| **c != NarrowbandChannel::Unknown && !ids.is_empty())
        .map(|(c, _)| *c)
        .collect();

    let (composition, confidence) = suggest_composition(&present, &observations);

    let channels: Vec<ChannelGroup> = channel_buckets
        .into_iter()
        .map(|(channel, asset_ids)| ChannelGroup { channel, asset_ids })
        .collect();

    NarrowbandAnalysis {
        channels,
        composition_suggestion: composition,
        confidence,
        observations,
    }
}

// ─── Channel resolution ────────────────────────────────────────

/// Resolve a single asset's narrowband channel. Returns
/// the resolved channel + the observation(s) that drove
/// the resolution.
fn resolve_channel(
    asset: &AssetRef<'_>,
) -> (Option<NarrowbandChannel>, Vec<NarrowbandObservation>) {
    let mut observations: Vec<NarrowbandObservation> = Vec::new();

    // Signal 1 — FILTER metadata. Strongest signal.
    let filter_channel = asset.metadata.filter.as_deref().map(normalise_filter);

    // Signal 2 — filename pattern. Weaker fallback when
    // FILTER is missing or unrecognised.
    let filename_channel = extract_channel_from_filename(asset.path);

    let resolved = match (filter_channel, filename_channel) {
        (Some(c), _) if c != NarrowbandChannel::Unknown => Some(c),
        (None, Some(c)) if c != NarrowbandChannel::Unknown => Some(c),
        // Filter was Unknown — fall through to filename
        // resolution. The Unknown bucket exists for
        // surfaces where neither resolves.
        (Some(NarrowbandChannel::Unknown), Some(c)) if c != NarrowbandChannel::Unknown => Some(c),
        (Some(c), _) => Some(c), // FILTER explicitly Unknown
        (None, Some(c)) => Some(c),
        (None, None) => None,
    };

    if let Some(filter) = asset.metadata.filter.as_deref() {
        let normalised = normalise_filter(filter);
        observations.push(NarrowbandObservation {
            signal: "filter_metadata".into(),
            value: filter.to_string(),
            weight: 0.7,
            confidence: if normalised != NarrowbandChannel::Unknown {
                0.95
            } else {
                0.50
            },
            resolves_to: if normalised == NarrowbandChannel::Unknown {
                None
            } else {
                Some(normalised)
            },
        });
    }

    if let Some(filename_channel) = filename_channel {
        observations.push(NarrowbandObservation {
            signal: "filename_pattern".into(),
            value: asset.path.to_string(),
            weight: 0.3,
            confidence: 0.75,
            resolves_to: if filename_channel == NarrowbandChannel::Unknown {
                None
            } else {
                Some(filename_channel)
            },
        });
    }

    (resolved, observations)
}

/// Normalise a FITS filter string into a
/// `NarrowbandChannel`. Returns `Unknown` for unrecognised
/// strings (never silently drops them).
fn normalise_filter(filter: &str) -> NarrowbandChannel {
    // Strip whitespace, hyphens, underscores. Lower-case
    // first so that the multi-byte Unicode alpha (α) is
    // folded to its lower form (which we then handle as
    // `hα` → matches the case below).
    let f = filter.trim().to_lowercase().replace(['-', '_', ' '], "");
    match f.as_str() {
        // Ha — including the Greek alpha variant.
        "ha" | "hα" | "halpha" | "halphaα" => NarrowbandChannel::Ha,
        // OIII
        "oiii" | "o3" | "o" => NarrowbandChannel::OIII,
        // SII
        "sii" | "s2" | "sulphur" | "sulfur" => NarrowbandChannel::SII,
        // Broadband
        "l" | "lum" | "luminance" => NarrowbandChannel::L,
        "r" | "red" => NarrowbandChannel::R,
        "g" | "green" => NarrowbandChannel::G,
        "b" | "blue" => NarrowbandChannel::B,
        // Continuum
        "clear" | "cont" | "continuum" | "uv" | "ir" | "ircut" | "irblock" => {
            NarrowbandChannel::Continuum
        }
        _ => NarrowbandChannel::Unknown,
    }
}

/// Extract a narrowband channel from a filename pattern.
/// Matches common conventions: `*_HA_*`, `*_OIII_*`,
/// `*-SHO-*`, etc. Returns `None` if no channel marker is
/// found.
fn extract_channel_from_filename(path: &str) -> Option<NarrowbandChannel> {
    let upper = path.to_uppercase();
    // Check narrowband markers first (more specific than
    // broadband).
    if upper.contains("HA") || upper.contains("HALPHA") {
        return Some(NarrowbandChannel::Ha);
    }
    if upper.contains("OIII") || upper.contains("O3") {
        return Some(NarrowbandChannel::OIII);
    }
    if upper.contains("SII") || upper.contains("S2") {
        return Some(NarrowbandChannel::SII);
    }
    // Broadband markers.
    if upper.contains("LUM") {
        return Some(NarrowbandChannel::L);
    }
    // Avoid matching "R"/"G"/"B" as standalone filenames
    // because they're too short — require a word boundary.
    for marker in &[
        "_R.", "-R.", "_R_", "-R-", "_G.", "-G.", "_G_", "-G-", "_B.", "-B.", "_B_", "-B-",
    ] {
        if upper.contains(marker) {
            // Determine which by checking the marker.
            let ch = if marker.contains('R') {
                NarrowbandChannel::R
            } else if marker.contains('G') {
                NarrowbandChannel::G
            } else {
                NarrowbandChannel::B
            };
            return Some(ch);
        }
    }
    None
}

// ─── Composition suggestion ────────────────────────────────────

/// Suggest a composition from the present channels.
///
/// Rules:
/// - All three narrowband (Ha + OIII + SII) → `HooOrSho`
///   (both palettes possible; the UI surfaces both).
/// - Ha + OIII only → `Hoo`.
/// - SII + Ha + OIII → `HooOrSho` (SHO takes precedence
///   for tri-colour).
/// - SII + OIII only → `Hoo` (Ha missing; degraded
///   Hubble).
/// - Single broadband (L/R/G/B/continuum) → `Lrgb` if
///   L present + any colour; `Mono` if only one channel.
/// - Only `L` (no colour) → `Lrgb` (L mono).
/// - Only one narrowband (Ha, OIII, or SII alone) →
///   `Mono`.
/// - `Unknown` only → `None` with low confidence.
fn suggest_composition(
    present: &[NarrowbandChannel],
    observations: &[NarrowbandObservation],
) -> (NarrowbandComposition, f64) {
    let has = |c: NarrowbandChannel| present.contains(&c);

    // Compute confidence as the weighted average of the
    // observations' confidences, normalised by total
    // weight.
    let (weighted, total_weight) = observations.iter().fold((0.0, 0.0), |(w, t), o| {
        if o.resolves_to.is_some() {
            (w + o.weight * o.confidence, t + o.weight)
        } else {
            (w, t)
        }
    });
    let confidence = if total_weight > 0.0 {
        weighted / total_weight
    } else {
        0.0
    };

    // Narrowband tri-colour.
    if has(NarrowbandChannel::Ha) && has(NarrowbandChannel::OIII) && has(NarrowbandChannel::SII) {
        return (NarrowbandComposition::HooOrSho, confidence);
    }
    // Narrowband bi-colour.
    if has(NarrowbandChannel::Ha) && has(NarrowbandChannel::OIII) {
        return (NarrowbandComposition::Hoo, confidence);
    }
    // SII + OIII (Ha missing) — degraded Hubble.
    if has(NarrowbandChannel::SII) && has(NarrowbandChannel::OIII) && !has(NarrowbandChannel::Ha) {
        return (NarrowbandComposition::Hoo, confidence);
    }

    // Broadband: L + at least one colour → LRGB.
    if has(NarrowbandChannel::L) {
        let colour_count = [
            NarrowbandChannel::R,
            NarrowbandChannel::G,
            NarrowbandChannel::B,
        ]
        .iter()
        .filter(|c| has(**c))
        .count();
        if colour_count >= 1 {
            return (NarrowbandComposition::Lrgb, confidence);
        }
        // L only.
        return (NarrowbandComposition::Mono, confidence);
    }
    // RGB only (no L) → RGB → Mono.
    let colour_count = [
        NarrowbandChannel::R,
        NarrowbandChannel::G,
        NarrowbandChannel::B,
    ]
    .iter()
    .filter(|c| has(**c))
    .count();
    if colour_count == 3 {
        return (NarrowbandComposition::Lrgb, confidence);
    }
    if colour_count >= 1 {
        return (NarrowbandComposition::Mono, confidence);
    }
    // Single narrowband (Ha, OIII, or SII alone).
    let narrowband_count = [
        NarrowbandChannel::Ha,
        NarrowbandChannel::OIII,
        NarrowbandChannel::SII,
    ]
    .iter()
    .filter(|c| has(**c))
    .count();
    if narrowband_count == 1 {
        return (NarrowbandComposition::Mono, confidence);
    }

    (NarrowbandComposition::None, confidence)
}

// ─── Unit tests ─────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn metadata(filter: Option<&str>) -> ExtractedMetadata {
        ExtractedMetadata {
            object: None,
            exptime: None,
            filter: filter.map(String::from),
            xbinning: None,
            ybinning: None,
            ccd_temp: None,
            naxis1: None,
            naxis2: None,
            bitpix: None,
            bayerpat: None,
            telescop: None,
            instrume: None,
            focallen: None,
            gain: None,
            offset: None,
            date_obs: None,
            width: None,
            height: None,
            bit_depth: None,
            camera: None,
        }
    }

    fn asset(id: &str, filter: Option<&str>, path: &str) -> AssetRef<'static> {
        // Leak the metadata so the AssetRef can carry a
        // 'static lifetime for these tests. Bounded by test
        // process lifetime.
        let m: &'static ExtractedMetadata = Box::leak(Box::new(metadata(filter)));
        AssetRef {
            asset_id: id.to_string(),
            metadata: m,
            path: Box::leak(path.to_string().into_boxed_str()),
        }
    }

    // ─── normalise_filter ───────────────────────────────────

    #[test]
    fn normalise_filter_handles_ha_variants() {
        assert_eq!(normalise_filter("Ha"), NarrowbandChannel::Ha);
        assert_eq!(normalise_filter("HA"), NarrowbandChannel::Ha);
        assert_eq!(normalise_filter("Hα"), NarrowbandChannel::Ha);
        assert_eq!(normalise_filter("Halpha"), NarrowbandChannel::Ha);
        assert_eq!(normalise_filter("H-ALPHA"), NarrowbandChannel::Ha);
        assert_eq!(normalise_filter("H_ALPHA"), NarrowbandChannel::Ha);
        assert_eq!(normalise_filter(" halpha "), NarrowbandChannel::Ha);
    }

    #[test]
    fn normalise_filter_handles_oiii_variants() {
        assert_eq!(normalise_filter("OIII"), NarrowbandChannel::OIII);
        assert_eq!(normalise_filter("oiii"), NarrowbandChannel::OIII);
        assert_eq!(normalise_filter("O3"), NarrowbandChannel::OIII);
        assert_eq!(normalise_filter("O-III"), NarrowbandChannel::OIII);
    }

    #[test]
    fn normalise_filter_handles_sii_variants() {
        assert_eq!(normalise_filter("SII"), NarrowbandChannel::SII);
        assert_eq!(normalise_filter("S2"), NarrowbandChannel::SII);
        assert_eq!(normalise_filter("Sulphur"), NarrowbandChannel::SII);
        assert_eq!(normalise_filter("S-II"), NarrowbandChannel::SII);
        assert_eq!(normalise_filter("Sulfur"), NarrowbandChannel::SII);
    }

    #[test]
    fn normalise_filter_handles_broadband() {
        assert_eq!(normalise_filter("L"), NarrowbandChannel::L);
        assert_eq!(normalise_filter("Lum"), NarrowbandChannel::L);
        assert_eq!(normalise_filter("Luminance"), NarrowbandChannel::L);
        assert_eq!(normalise_filter("R"), NarrowbandChannel::R);
        assert_eq!(normalise_filter("Red"), NarrowbandChannel::R);
        assert_eq!(normalise_filter("G"), NarrowbandChannel::G);
        assert_eq!(normalise_filter("Green"), NarrowbandChannel::G);
        assert_eq!(normalise_filter("B"), NarrowbandChannel::B);
        assert_eq!(normalise_filter("Blue"), NarrowbandChannel::B);
    }

    #[test]
    fn normalise_filter_handles_continuum() {
        assert_eq!(normalise_filter("Clear"), NarrowbandChannel::Continuum);
        assert_eq!(normalise_filter("CLEAR"), NarrowbandChannel::Continuum);
        assert_eq!(normalise_filter("UV"), NarrowbandChannel::Continuum);
        assert_eq!(normalise_filter("IR"), NarrowbandChannel::Continuum);
        assert_eq!(normalise_filter("IR-cut"), NarrowbandChannel::Continuum);
    }

    #[test]
    fn normalise_filter_unknown_for_unrecognised() {
        assert_eq!(normalise_filter("Foo"), NarrowbandChannel::Unknown);
        assert_eq!(normalise_filter(""), NarrowbandChannel::Unknown);
        assert_eq!(normalise_filter("   "), NarrowbandChannel::Unknown);
    }

    // ─── filename extraction ─────────────────────────────────

    #[test]
    fn filename_extraction_finds_ha() {
        assert_eq!(
            extract_channel_from_filename("/data/NGC7000_HA_001.fits"),
            Some(NarrowbandChannel::Ha)
        );
        assert_eq!(
            extract_channel_from_filename("/data/NGC7000_Ha_001.fits"),
            Some(NarrowbandChannel::Ha)
        );
        assert_eq!(
            extract_channel_from_filename("/data/NGC7000_HALPHA_001.fits"),
            Some(NarrowbandChannel::Ha)
        );
    }

    #[test]
    fn filename_extraction_finds_oiii() {
        assert_eq!(
            extract_channel_from_filename("/data/NGC7000_OIII_001.fits"),
            Some(NarrowbandChannel::OIII)
        );
        assert_eq!(
            extract_channel_from_filename("/data/NGC7000_O3_001.fits"),
            Some(NarrowbandChannel::OIII)
        );
    }

    #[test]
    fn filename_extraction_finds_sii() {
        assert_eq!(
            extract_channel_from_filename("/data/NGC7000_SII_001.fits"),
            Some(NarrowbandChannel::SII)
        );
        assert_eq!(
            extract_channel_from_filename("/data/NGC7000_S2_001.fits"),
            Some(NarrowbandChannel::SII)
        );
    }

    #[test]
    fn filename_extraction_finds_lum() {
        assert_eq!(
            extract_channel_from_filename("/data/NGC7000_LUM_001.fits"),
            Some(NarrowbandChannel::L)
        );
    }

    #[test]
    fn filename_extraction_finds_rgb_with_boundaries() {
        assert_eq!(
            extract_channel_from_filename("/data/M31_R_001.fits"),
            Some(NarrowbandChannel::R)
        );
        assert_eq!(
            extract_channel_from_filename("/data/M31_G_001.fits"),
            Some(NarrowbandChannel::G)
        );
        assert_eq!(
            extract_channel_from_filename("/data/M31_B_001.fits"),
            Some(NarrowbandChannel::B)
        );
    }

    #[test]
    fn filename_extraction_returns_none_for_neutral() {
        assert_eq!(extract_channel_from_filename("/data/M31_001.fits"), None);
        assert_eq!(extract_channel_from_filename(""), None);
    }

    #[test]
    fn filename_extraction_returns_none_for_broadband_r_no_boundary() {
        // A filename without R/G/B word boundary doesn't
        // match (avoids false positives like "RED_GIANT_*"
        // matching on "R").
        assert_eq!(
            extract_channel_from_filename("/data/RED_GIANT_001.fits"),
            None
        );
    }

    // ─── Composition suggestion ─────────────────────────────

    #[test]
    fn three_narrowband_channels_suggest_hoo_or_sho() {
        let (comp, _) = suggest_composition(
            &[
                NarrowbandChannel::Ha,
                NarrowbandChannel::OIII,
                NarrowbandChannel::SII,
            ],
            &[],
        );
        assert_eq!(comp, NarrowbandComposition::HooOrSho);
    }

    #[test]
    fn ha_and_oiii_suggest_hoo() {
        let (comp, _) = suggest_composition(&[NarrowbandChannel::Ha, NarrowbandChannel::OIII], &[]);
        assert_eq!(comp, NarrowbandComposition::Hoo);
    }

    #[test]
    fn sii_and_oiii_without_ha_suggest_hoo() {
        let (comp, _) =
            suggest_composition(&[NarrowbandChannel::SII, NarrowbandChannel::OIII], &[]);
        assert_eq!(comp, NarrowbandComposition::Hoo);
    }

    #[test]
    fn single_narrowband_suggests_mono() {
        let (comp, _) = suggest_composition(&[NarrowbandChannel::Ha], &[]);
        assert_eq!(comp, NarrowbandComposition::Mono);
        let (comp, _) = suggest_composition(&[NarrowbandChannel::OIII], &[]);
        assert_eq!(comp, NarrowbandComposition::Mono);
        let (comp, _) = suggest_composition(&[NarrowbandChannel::SII], &[]);
        assert_eq!(comp, NarrowbandComposition::Mono);
    }

    #[test]
    fn l_plus_rgb_suggests_lrgb() {
        let (comp, _) = suggest_composition(
            &[
                NarrowbandChannel::L,
                NarrowbandChannel::R,
                NarrowbandChannel::G,
                NarrowbandChannel::B,
            ],
            &[],
        );
        assert_eq!(comp, NarrowbandComposition::Lrgb);
    }

    #[test]
    fn l_plus_one_colour_suggests_lrgb() {
        let (comp, _) = suggest_composition(&[NarrowbandChannel::L, NarrowbandChannel::R], &[]);
        assert_eq!(comp, NarrowbandComposition::Lrgb);
    }

    #[test]
    fn rgb_only_suggests_lrgb() {
        let (comp, _) = suggest_composition(
            &[
                NarrowbandChannel::R,
                NarrowbandChannel::G,
                NarrowbandChannel::B,
            ],
            &[],
        );
        assert_eq!(comp, NarrowbandComposition::Lrgb);
    }

    #[test]
    fn single_rgb_channel_suggests_mono() {
        let (comp, _) = suggest_composition(&[NarrowbandChannel::R], &[]);
        assert_eq!(comp, NarrowbandComposition::Mono);
    }

    #[test]
    fn l_only_suggests_mono() {
        let (comp, _) = suggest_composition(&[NarrowbandChannel::L], &[]);
        assert_eq!(comp, NarrowbandComposition::Mono);
    }

    #[test]
    fn no_channels_suggests_none() {
        let (comp, conf) = suggest_composition(&[], &[]);
        assert_eq!(comp, NarrowbandComposition::None);
        assert_eq!(conf, 0.0);
    }

    #[test]
    fn confidence_from_filter_observations() {
        let obs = vec![NarrowbandObservation {
            signal: "filter_metadata".into(),
            value: "Ha".into(),
            weight: 0.7,
            confidence: 0.95,
            resolves_to: Some(NarrowbandChannel::Ha),
        }];
        let (_comp, conf) = suggest_composition(&[NarrowbandChannel::Ha], &obs);
        assert!((conf - 0.95).abs() < 0.001);
    }

    // ─── Full detect_narrowband ─────────────────────────────

    #[test]
    fn empty_input_yields_none() {
        let a = detect_narrowband(&[]);
        assert_eq!(a.composition_suggestion, NarrowbandComposition::None);
        assert_eq!(a.confidence, 0.0);
        assert!(a.channels.is_empty());
    }

    #[test]
    fn spec_example_ngc7000_hoo() {
        // The §11 example: Ha 124 + OIII 108 → HOO.
        let mut assets = Vec::new();
        for i in 0..124 {
            assets.push(asset(
                &format!("ha_{i:03}"),
                Some("Ha"),
                &format!("/data/NGC7000/HA_{i:03}.fits"),
            ));
        }
        for i in 0..108 {
            assets.push(asset(
                &format!("oiii_{i:03}"),
                Some("OIII"),
                &format!("/data/NGC7000/OIII_{i:03}.fits"),
            ));
        }
        let a = detect_narrowband(&assets);
        assert_eq!(a.composition_suggestion, NarrowbandComposition::Hoo);
        // 2 channels.
        assert_eq!(a.channels.len(), 2);
        let ha = a
            .channels
            .iter()
            .find(|c| c.channel == NarrowbandChannel::Ha)
            .unwrap();
        assert_eq!(ha.asset_ids.len(), 124);
        let oiii = a
            .channels
            .iter()
            .find(|c| c.channel == NarrowbandChannel::OIII)
            .unwrap();
        assert_eq!(oiii.asset_ids.len(), 108);
    }

    #[test]
    fn three_channel_sho_possible() {
        let mut assets = Vec::new();
        for i in 0..124 {
            assets.push(asset(&format!("ha_{i}"), Some("Ha"), "/x.fits"));
        }
        for i in 0..108 {
            assets.push(asset(&format!("oiii_{i}"), Some("OIII"), "/x.fits"));
        }
        for i in 0..90 {
            assets.push(asset(&format!("sii_{i}"), Some("SII"), "/x.fits"));
        }
        let a = detect_narrowband(&assets);
        assert_eq!(a.composition_suggestion, NarrowbandComposition::HooOrSho);
    }

    #[test]
    fn filename_fallback_when_filter_missing() {
        let assets = vec![
            asset("a1", None, "/data/NGC7000_HA_001.fits"),
            asset("a2", None, "/data/NGC7000_HA_002.fits"),
            asset("a3", None, "/data/NGC7000_OIII_001.fits"),
        ];
        let a = detect_narrowband(&assets);
        assert_eq!(a.composition_suggestion, NarrowbandComposition::Hoo);
    }

    #[test]
    fn unknown_filter_falls_through_to_filename() {
        let assets = vec![asset("a1", Some("Foo"), "/data/NGC7000_HA_001.fits")];
        let a = detect_narrowband(&assets);
        // Filter metadata resolved to Unknown; filename
        // resolves to Ha.
        let ha = a
            .channels
            .iter()
            .find(|c| c.channel == NarrowbandChannel::Ha)
            .unwrap();
        assert_eq!(ha.asset_ids.len(), 1);
        // Also an Unknown bucket for the asset id, but the
        // bucket logic only adds to Unknown when no other
        // resolution succeeded. In this case filename
        // resolved to Ha, so the asset goes into Ha, not
        // Unknown. Verify that.
        let unknown = a
            .channels
            .iter()
            .find(|c| c.channel == NarrowbandChannel::Unknown);
        assert!(unknown.is_none() || unknown.unwrap().asset_ids.is_empty());
    }

    #[test]
    fn unknown_only_yields_none_with_low_confidence() {
        let assets = vec![
            asset("a1", Some("Foo"), "/data/M31_001.fits"),
            asset("a2", Some("Bar"), "/data/M31_002.fits"),
        ];
        let a = detect_narrowband(&assets);
        assert_eq!(a.composition_suggestion, NarrowbandComposition::None);
        // Confidence is low (Unknown filter observations
        // contribute 0.5 confidence * 0.7 weight; filename
        // extraction returned None → no observation).
        assert!(a.confidence < 0.7);
    }

    #[test]
    fn case_insensitive_filter() {
        let assets = vec![
            asset("a1", Some("ha"), "/x.fits"),
            asset("a2", Some("HA"), "/x.fits"),
            asset("a3", Some("Ha"), "/x.fits"),
            asset("a4", Some("h-alpha"), "/x.fits"),
        ];
        let a = detect_narrowband(&assets);
        let ha = a
            .channels
            .iter()
            .find(|c| c.channel == NarrowbandChannel::Ha)
            .unwrap();
        assert_eq!(ha.asset_ids.len(), 4);
    }

    #[test]
    fn lrgb_session_classifies_correctly() {
        let assets = vec![
            asset("lum1", Some("L"), "/x.fits"),
            asset("lum2", Some("L"), "/x.fits"),
            asset("r1", Some("R"), "/x.fits"),
            asset("g1", Some("G"), "/x.fits"),
            asset("b1", Some("B"), "/x.fits"),
        ];
        let a = detect_narrowband(&assets);
        assert_eq!(a.composition_suggestion, NarrowbandComposition::Lrgb);
        assert_eq!(a.channels.len(), 4); // L, R, G, B
    }

    #[test]
    fn narrowband_channel_serialises_snake_case() {
        assert_eq!(
            serde_json::to_string(&NarrowbandChannel::Ha).unwrap(),
            "\"ha\""
        );
        assert_eq!(
            serde_json::to_string(&NarrowbandChannel::OIII).unwrap(),
            "\"oiii\""
        );
        assert_eq!(
            serde_json::to_string(&NarrowbandChannel::Continuum).unwrap(),
            "\"continuum\""
        );
    }

    #[test]
    fn narrowband_composition_serialises_snake_case() {
        assert_eq!(
            serde_json::to_string(&NarrowbandComposition::HooOrSho).unwrap(),
            "\"hoo_or_sho\""
        );
        assert_eq!(
            serde_json::to_string(&NarrowbandComposition::Lrgb).unwrap(),
            "\"lrgb\""
        );
    }

    #[test]
    fn observations_are_recorded() {
        let assets = vec![asset("a1", Some("Ha"), "/data/NGC7000_HA_001.fits")];
        let a = detect_narrowband(&assets);
        // Two signals: filter_metadata (Ha) +
        // filename_pattern (HA in filename).
        assert!(a.observations.len() >= 2);
        let signals: Vec<&str> = a.observations.iter().map(|o| o.signal.as_str()).collect();
        assert!(signals.contains(&"filter_metadata"));
        assert!(signals.contains(&"filename_pattern"));
    }

    #[test]
    fn asset_path_extraction_handles_full_paths() {
        let assets = vec![asset("a1", None, "/home/user/data/M31/Ha/light_001.fits")];
        let a = detect_narrowband(&assets);
        let ha = a
            .channels
            .iter()
            .find(|c| c.channel == NarrowbandChannel::Ha)
            .unwrap();
        assert_eq!(ha.asset_ids.len(), 1);
    }
}
