//! CR-07 §25 — Comparison data model.
//!
//! Establishes the type system that backs the Image Review, Comparison
//! & Decision Workspace. The 8 §25 types are:
//!
//! - [`ComparisonSession`] — a user's active comparison activity
//! - [`ComparisonItem`] — references an Image Version (one slot in a
//!   session)
//! - [`ComparisonRegion`] — defines the comparison area (whole image,
//!   selected region, or specific feature)
//! - [`ComparisonMetric`] — measured characteristics for a region
//! - [`ComparisonDelta`] — direction + magnitude between two metrics
//! - [`QualityAssessment`] — analytical observations (warnings +
//!   summary)
//! - [`ImageDecision`] — decision state (Working / Candidate /
//!   Preferred / Final / Rejected / Reference)
//! - [`ComparisonSet`] — reusable collection of candidate versions
//!
//! ## Design notes
//!
//! **No behavior in this slice.** B1 Foundation is paperwork + types
//! only. Behaviour lives in B2 (metrics + delta), B3 (decisions), and
//! B4 (UX). The persistence layer is intentionally absent here —
//! the audit's recommendation is to wire these into the existing
//! `db.rs` sqlite connection when B3 ships, not to introduce a
//! parallel persistence crate.
//!
//! **The §25 types mirror the CR-07 §25 vocabulary.** Field names
//! match the spec so consumers can map one-to-one without translation
//! tables.
//!
//! **Comparison is non-destructive (ADR-07.4).** No method on any
//! type here mutates or deletes the referenced Image Version.
//!
//! **The user owns the decision (ADR-07.7).** [`ImageDecision::promote`]
//! returns the next state in the workflow but never auto-promotes.
//!
//! **Timestamps are ISO-8601 strings**, matching the convention used
//! by `domain.rs::ImageVersion` and other domain types (see
//! `created_at: String` throughout the codebase).

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Returns an ISO-8601 (RFC 3339) timestamp for the current instant.
/// Wrapped in a helper to keep the public API ergonomic without
/// pulling in `chrono` (the codebase convention is `String`).
pub fn now_iso8601() -> String {
    // std::time::SystemTime is the only datetime primitive in `core`
    // without adding a new dependency. RFC 3339 formatting is
    // hand-rolled; this slice does not need sub-second precision and
    // a tiny helper is cheaper than a new dependency.
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    format_iso8601_utc(secs)
}

/// Format a Unix timestamp as RFC 3339 UTC (e.g. "2026-09-12T07:00:00Z").
/// Hand-rolled to avoid a new `chrono` dependency. Valid for the
/// range 1970-2100; beyond that, calendar arithmetic drifts (matches
/// the codebase's deprecation story).
fn format_iso8601_utc(secs: u64) -> String {
    let days = (secs / 86_400) as i64;
    let secs_of_day = (secs % 86_400) as u32;
    let hour = secs_of_day / 3600;
    let minute = (secs_of_day % 3600) / 60;
    let second = secs_of_day % 60;
    let (year, month, day) = civil_from_days(days);
    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
        year, month, day, hour, minute, second
    )
}

/// Howard Hinnant's `civil_from_days` algorithm — converts a Unix day
/// count to (year, month, day). Public-domain reference:
/// https://howardhinnant.github.io/date_algorithms.html
fn civil_from_days(z: i64) -> (i32, u32, u32) {
    let z = z + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    let y = if m <= 2 { y + 1 } else { y };
    (y as i32, m, d)
}

/// The user's active comparison activity. Bounded to one project.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ComparisonSession {
    pub id: String,
    pub project_id: String,
    pub created_at: String,
    pub mode: ComparisonMode,
    pub items: Vec<ComparisonItem>,
    pub regions: Vec<ComparisonRegion>,
    pub metrics: Vec<ComparisonMetric>,
    pub assessment: Option<QualityAssessment>,
}

impl ComparisonSession {
    /// Convenience constructor. Empty item/region/metric vectors.
    pub fn new(project_id: impl Into<String>, mode: ComparisonMode) -> Self {
        Self {
            id: format!("cmp-{}", now_iso8601().replace([':', '-', 'T', 'Z'], "")),
            project_id: project_id.into(),
            created_at: now_iso8601(),
            mode,
            items: Vec::new(),
            regions: Vec::new(),
            metrics: Vec::new(),
            assessment: None,
        }
    }
}

/// Comparison mode — selects which rendering the workspace applies.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ComparisonMode {
    SideBySide,
    Split,
    Blink,
    DifferenceAbsolute,
    DifferenceSigned,
    DifferenceAmplified,
    DifferenceStructural,
    Overlay,
}

impl ComparisonMode {
    pub fn label(self) -> &'static str {
        match self {
            ComparisonMode::SideBySide => "Side-by-side",
            ComparisonMode::Split => "Split view",
            ComparisonMode::Blink => "Blink",
            ComparisonMode::DifferenceAbsolute => "Difference (absolute)",
            ComparisonMode::DifferenceSigned => "Difference (signed)",
            ComparisonMode::DifferenceAmplified => "Difference (amplified)",
            ComparisonMode::DifferenceStructural => "Difference (structural)",
            ComparisonMode::Overlay => "Overlay",
        }
    }
}

/// A reference to an Image Version within a comparison session.
/// Slots are addressable by `slot` ("a" / "b" / "c" / "d"); the
/// `version_id` is the durable Image Version ID from the project.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ComparisonItem {
    pub slot: ComparisonSlot,
    pub version_id: String,
    pub label: Option<String>,
}

/// A/B/C/D slot in the comparison layout.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum ComparisonSlot {
    A,
    B,
    C,
    D,
}

impl ComparisonSlot {
    pub fn letter(self) -> char {
        match self {
            ComparisonSlot::A => 'A',
            ComparisonSlot::B => 'B',
            ComparisonSlot::C => 'C',
            ComparisonSlot::D => 'D',
        }
    }
}

/// Defines the comparison area.
///
/// WholeImage is implicit when no region is set; SelectedRegion
/// requires shape data (polygon, rectangle, or circle); SpecificFeature
/// references a semantic feature recognized by AstroForge (galaxy
/// core, nebula, star field).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ComparisonRegion {
    pub id: String,
    pub session_id: String,
    pub scope: ComparisonScope,
    /// For SelectedRegion: shape geometry in normalized image
    /// coordinates (0..1).
    pub shape: Option<RegionShape>,
    /// For SpecificFeature: feature identifier (e.g.
    /// "galaxy_core", "nebula", "star_field", "background").
    pub feature: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ComparisonScope {
    WholeImage,
    SelectedRegion,
    SpecificFeature,
}

/// Region geometry in normalized image coordinates.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum RegionShape {
    Rectangle { x: f64, y: f64, w: f64, h: f64 },
    Circle { cx: f64, cy: f64, r: f64 },
    Polygon { points: Vec<(f64, f64)> },
}

/// A measured characteristic for a region within a session.
///
/// `values` is keyed by [`MetricKind`] (stringified); values are
/// deliberately `f64` rather than a rich enum because §8 lists 16+
/// metric kinds and adding variants as enums would couple this type
/// to the metric registry.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ComparisonMetric {
    pub id: String,
    pub session_id: String,
    pub region_id: String,
    pub version_id: String,
    /// `MetricKind::as_str()` keys (e.g. "noise.luminance", "stars.count",
    /// "background.gradient").
    pub values: BTreeMap<String, f64>,
}

/// Difference direction between two metrics in the same session.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DeltaDirection {
    /// A's value improved relative to B (e.g. lower noise).
    Improved,
    Degraded,
    Unchanged,
    /// The change is too small to be material or the direction is
    /// ambiguous (e.g. higher local contrast is not unambiguously
    /// better).
    Inconclusive,
}

impl DeltaDirection {
    /// Render as the §10 directional indicator (✓ improved,
    /// ⚠ degraded, — unchanged, ? inconclusive).
    pub fn indicator(self) -> &'static str {
        match self {
            DeltaDirection::Improved => "improved",
            DeltaDirection::Degraded => "degraded",
            DeltaDirection::Unchanged => "unchanged",
            DeltaDirection::Inconclusive => "inconclusive",
        }
    }
}

/// Difference between two metric values: direction + percent change.
///
/// `baseline_version_id` is the version the delta is measured against;
/// `compared_version_id` is the candidate.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ComparisonDelta {
    pub id: String,
    pub session_id: String,
    pub metric_id: String,
    pub baseline_version_id: String,
    pub compared_version_id: String,
    pub direction: DeltaDirection,
    /// Percent change (negative when the candidate's value is lower
    /// than baseline's, regardless of whether lower is better).
    pub percent_change: f64,
    /// Materiality threshold: |percent_change| below this is
    /// classified `Unchanged`.
    pub materiality_threshold: f64,
}

impl ComparisonDelta {
    /// Compute direction + percent change. `improvement_sign` is `-1`
    /// when lower-is-better (noise, clipping) and `+1` when
    /// higher-is-better (sharpness, SNR). For metrics where direction
    /// is ambiguous (e.g. local contrast), pass `0` to force
    /// `Inconclusive`.
    pub fn compute(
        baseline: f64,
        compared: f64,
        improvement_sign: i8,
        materiality_threshold: f64,
    ) -> DeltaDirection {
        if baseline.abs() < f64::EPSILON {
            return DeltaDirection::Inconclusive;
        }
        let percent = (compared - baseline) / baseline.abs() * 100.0;
        if percent.abs() < materiality_threshold {
            return DeltaDirection::Unchanged;
        }
        if improvement_sign == 0 {
            return DeltaDirection::Inconclusive;
        }
        // positive percent means candidate is higher; improvement_sign
        // tells us whether higher is better.
        let candidate_is_better = (percent > 0.0) == (improvement_sign > 0);
        if candidate_is_better {
            DeltaDirection::Improved
        } else {
            DeltaDirection::Degraded
        }
    }
}

/// Analytical observations for a comparison session.
///
/// `findings` are aggregated from `quality_gates::GateFinding` rows
/// keyed by `GateId`; `integrity_checks` are the §12 astronomical
/// integrity findings (faint-structure suppression, star
/// disappearance, halos, ringing, etc.).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct QualityAssessment {
    pub id: String,
    pub session_id: String,
    pub summary: String,
    pub findings: Vec<AssessmentFinding>,
    pub integrity_checks: Vec<IntegrityCheck>,
    /// Optional §11 natural-language assessment (e.g. "Strong
    /// improvement with minor trade-offs.").
    pub verdict: Option<QualityVerdict>,
}

impl QualityAssessment {
    pub fn new(session_id: impl Into<String>) -> Self {
        Self {
            id: format!("qa-{}", now_iso8601().replace([':', '-', 'T', 'Z'], "")),
            session_id: session_id.into(),
            ..Default::default()
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AssessmentFinding {
    pub kind: String,
    pub severity: AssessmentSeverity,
    pub message: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum AssessmentSeverity {
    Info,
    Warn,
    Fail,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct IntegrityCheck {
    pub kind: String,
    pub passed: bool,
    pub detail: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum QualityVerdict {
    StrongImprovement,
    ImprovementWithTradeoffs,
    Neutral,
    Degradation,
    Inconclusive,
}

/// Decision state for an Image Version (§17).
///
/// The state machine is `Working → Candidate → Preferred → Final` with
/// branches to `Rejected` and `Reference`. History is preserved (each
/// `promote()` call appends to `history`); per ADR-08.4, no prior
/// state is rewritten.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum ImageDecisionState {
    Working,
    Candidate,
    Preferred,
    Final,
    Rejected,
    Reference,
}

impl ImageDecisionState {
    /// Returns the next state in the canonical promotion flow, or
    /// `None` if already terminal.
    pub fn next(self) -> Option<Self> {
        match self {
            ImageDecisionState::Working => Some(ImageDecisionState::Candidate),
            ImageDecisionState::Candidate => Some(ImageDecisionState::Preferred),
            ImageDecisionState::Preferred => Some(ImageDecisionState::Final),
            // Final / Rejected / Reference are terminal.
            ImageDecisionState::Final
            | ImageDecisionState::Rejected
            | ImageDecisionState::Reference => None,
        }
    }

    /// Can the user transition from this state to `target`?
    /// Terminal states (Final / Rejected / Reference) cannot be
    /// promoted. Reject is allowed from any non-terminal state.
    pub fn can_promote_to(self, target: Self) -> bool {
        match (self, target) {
            (ImageDecisionState::Working, ImageDecisionState::Candidate) => true,
            (ImageDecisionState::Candidate, ImageDecisionState::Preferred) => true,
            (ImageDecisionState::Preferred, ImageDecisionState::Final) => true,
            // Reject is allowed from Working / Candidate / Preferred.
            (
                ImageDecisionState::Working
                | ImageDecisionState::Candidate
                | ImageDecisionState::Preferred,
                ImageDecisionState::Rejected,
            ) => true,
            // No re-promotion; no skipping states.
            _ => false,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            ImageDecisionState::Working => "Working",
            ImageDecisionState::Candidate => "Candidate",
            ImageDecisionState::Preferred => "Preferred",
            ImageDecisionState::Final => "Final",
            ImageDecisionState::Rejected => "Rejected",
            ImageDecisionState::Reference => "Reference",
        }
    }
}

/// The decision record for a single Image Version, including the
/// promotion history (per ADR-08.4 historical execution is immutable).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ImageDecision {
    pub version_id: String,
    pub state: ImageDecisionState,
    pub history: Vec<DecisionHistoryEntry>,
    pub decided_at: String,
}

impl ImageDecision {
    /// New decision in the `Working` state with a single history
    /// entry.
    pub fn new(version_id: impl Into<String>) -> Self {
        let now = now_iso8601();
        Self {
            version_id: version_id.into(),
            state: ImageDecisionState::Working,
            history: vec![DecisionHistoryEntry {
                from: None,
                to: ImageDecisionState::Working,
                at: now.clone(),
                reason: None,
            }],
            decided_at: now,
        }
    }

    /// Promote to the canonical next state. Returns the new state on
    /// success or an error if the current state is terminal or the
    /// transition is not permitted.
    pub fn promote(
        &mut self,
        reason: Option<String>,
    ) -> Result<ImageDecisionState, PromotionError> {
        let target = self
            .state
            .next()
            .ok_or(PromotionError::Terminal(self.state))?;
        self.transition_to(target, reason)
    }

    /// Move to an arbitrary target state, validating the transition
    /// per [`ImageDecisionState::can_promote_to`].
    pub fn transition_to(
        &mut self,
        target: ImageDecisionState,
        reason: Option<String>,
    ) -> Result<ImageDecisionState, PromotionError> {
        if !self.state.can_promote_to(target) {
            return Err(PromotionError::InvalidTransition {
                from: self.state,
                to: target,
            });
        }
        let now = now_iso8601();
        self.history.push(DecisionHistoryEntry {
            from: Some(self.state),
            to: target,
            at: now.clone(),
            reason,
        });
        self.state = target;
        self.decided_at = now;
        Ok(target)
    }

    /// Mark as Rejected (allowed from any non-Final state).
    pub fn reject(&mut self, reason: Option<String>) -> Result<(), PromotionError> {
        if self.state == ImageDecisionState::Final {
            return Err(PromotionError::Terminal(self.state));
        }
        self.transition_to(ImageDecisionState::Rejected, reason)
            .map(|_| ())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DecisionHistoryEntry {
    pub from: Option<ImageDecisionState>,
    pub to: ImageDecisionState,
    pub at: String,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum PromotionError {
    /// The current state is terminal; no further promotion is
    /// permitted.
    Terminal(ImageDecisionState),
    /// The requested transition is not in the canonical promotion
    /// flow.
    InvalidTransition {
        from: ImageDecisionState,
        to: ImageDecisionState,
    },
}

impl std::fmt::Display for PromotionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PromotionError::Terminal(s) => {
                write!(f, "Image version is in terminal state {:?}", s)
            }
            PromotionError::InvalidTransition { from, to } => {
                write!(f, "Invalid promotion {:?} -> {:?}", from, to)
            }
        }
    }
}

impl std::error::Error for PromotionError {}

/// A reusable collection of candidate Image Versions (§16).
///
/// Example: "M42 Final Candidates" containing A — Natural / B — AI
/// Enhanced / C — High Detail / D — Narrowband Blend.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ComparisonSet {
    pub id: String,
    pub project_id: String,
    pub name: String,
    pub version_ids: Vec<String>,
    pub created_at: String,
    /// Optional human-readable label per slot, e.g.
    /// "A — Natural", "B — AI Enhanced".
    pub slot_labels: Vec<String>,
}

impl ComparisonSet {
    pub fn new(
        project_id: impl Into<String>,
        name: impl Into<String>,
        version_ids: Vec<String>,
    ) -> Self {
        Self {
            id: format!("set-{}", now_iso8601().replace([':', '-', 'T', 'Z'], "")),
            project_id: project_id.into(),
            name: name.into(),
            version_ids,
            created_at: now_iso8601(),
            slot_labels: Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decision_state_promotion_flow_is_canonical() {
        let mut d = ImageDecision::new("v1");
        assert_eq!(d.state, ImageDecisionState::Working);
        assert_eq!(d.promote(None).unwrap(), ImageDecisionState::Candidate);
        assert_eq!(d.promote(None).unwrap(), ImageDecisionState::Preferred);
        assert_eq!(d.promote(None).unwrap(), ImageDecisionState::Final);
        // Final is terminal.
        assert!(matches!(
            d.promote(None),
            Err(PromotionError::Terminal(ImageDecisionState::Final))
        ));
    }

    #[test]
    fn decision_history_preserves_full_lineage() {
        let mut d = ImageDecision::new("v1");
        d.promote(Some("noise improved".into())).unwrap();
        d.promote(Some("sharpness good".into())).unwrap();
        assert_eq!(d.history.len(), 3); // initial Working + 2 promotions
        assert_eq!(d.history[0].from, None);
        assert_eq!(d.history[0].to, ImageDecisionState::Working);
        assert_eq!(d.history[1].from, Some(ImageDecisionState::Working));
        assert_eq!(d.history[2].from, Some(ImageDecisionState::Candidate));
    }

    #[test]
    fn decision_rejects_skipping_states() {
        let mut d = ImageDecision::new("v1");
        // Cannot skip from Working to Final.
        assert!(matches!(
            d.transition_to(ImageDecisionState::Final, None),
            Err(PromotionError::InvalidTransition { .. })
        ));
        // State must remain unchanged on invalid transition.
        assert_eq!(d.state, ImageDecisionState::Working);
    }

    #[test]
    fn decision_can_be_rejected_from_any_non_final_state() {
        let mut d = ImageDecision::new("v1");
        d.promote(None).unwrap(); // Candidate
        d.reject(Some("noise too high".into())).unwrap();
        assert_eq!(d.state, ImageDecisionState::Rejected);
    }

    #[test]
    fn final_cannot_be_rejected() {
        let mut d = ImageDecision::new("v1");
        d.promote(None).unwrap();
        d.promote(None).unwrap();
        d.promote(None).unwrap(); // Final
        assert!(matches!(
            d.reject(None),
            Err(PromotionError::Terminal(ImageDecisionState::Final))
        ));
    }

    #[test]
    fn delta_direction_respects_improvement_sign() {
        // Lower is better for noise.
        assert_eq!(
            ComparisonDelta::compute(20.0, 10.0, -1, 5.0),
            DeltaDirection::Improved
        );
        assert_eq!(
            ComparisonDelta::compute(10.0, 20.0, -1, 5.0),
            DeltaDirection::Degraded
        );
        // Higher is better for sharpness.
        assert_eq!(
            ComparisonDelta::compute(3.0, 4.0, 1, 5.0),
            DeltaDirection::Improved
        );
    }

    #[test]
    fn delta_direction_handles_materiality_threshold() {
        // 1% change with 5% threshold -> Unchanged.
        assert_eq!(
            ComparisonDelta::compute(100.0, 101.0, -1, 5.0),
            DeltaDirection::Unchanged
        );
    }

    #[test]
    fn delta_direction_is_inconclusive_for_ambiguous_metrics() {
        // Local contrast: direction unknown.
        assert_eq!(
            ComparisonDelta::compute(0.5, 0.7, 0, 5.0),
            DeltaDirection::Inconclusive
        );
    }

    #[test]
    fn delta_direction_handles_zero_baseline() {
        // Zero baseline: percent change undefined.
        assert_eq!(
            ComparisonDelta::compute(0.0, 1.0, -1, 5.0),
            DeltaDirection::Inconclusive
        );
    }

    #[test]
    fn comparison_session_new_has_empty_collections() {
        let s = ComparisonSession::new("project-1", ComparisonMode::Split);
        assert!(s.items.is_empty());
        assert!(s.regions.is_empty());
        assert!(s.metrics.is_empty());
        assert!(s.assessment.is_none());
        assert_eq!(s.project_id, "project-1");
        assert_eq!(s.mode, ComparisonMode::Split);
    }

    #[test]
    fn comparison_set_round_trip() {
        let s = ComparisonSet::new(
            "m42-final",
            "M42 Final Candidates",
            vec!["v1".into(), "v2".into(), "v3".into()],
        );
        assert_eq!(s.version_ids.len(), 3);
        assert_eq!(s.slot_labels.len(), 0); // unset by default
    }

    #[test]
    fn comparison_mode_label_covers_all_variants() {
        // Defensive: ensures label() is updated when a new variant is
        // added.
        for mode in [
            ComparisonMode::SideBySide,
            ComparisonMode::Split,
            ComparisonMode::Blink,
            ComparisonMode::DifferenceAbsolute,
            ComparisonMode::DifferenceSigned,
            ComparisonMode::DifferenceAmplified,
            ComparisonMode::DifferenceStructural,
            ComparisonMode::Overlay,
        ] {
            assert!(!mode.label().is_empty());
        }
    }

    #[test]
    fn region_shape_normalized_coordinates() {
        // Defensive: shape values are 0..1 normalized. The type does
        // not enforce this at construction (serde-friendly); a future
        // validator should.
        let r = RegionShape::Rectangle {
            x: 0.25,
            y: 0.5,
            w: 0.1,
            h: 0.1,
        };
        if let RegionShape::Rectangle { x, y, w, h } = r {
            assert_eq!((x, y, w, h), (0.25, 0.5, 0.1, 0.1));
        } else {
            panic!("expected Rectangle");
        }
    }
}
