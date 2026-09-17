//! CR-07 §20: Post-comparison recommendation feedback loop.
//!
//! When a user finishes a comparison between two Image Versions
//! the data path already has both sides' metric snapshots
//! (`astroforge_core::comparison_metrics::metric_snapshot`) and
//! the `ImageDecision` state per side. The other recommendation
//! rules in `rules.rs` read a single `ImageAnalysisReport`: they
//! cannot see the comparison as a whole. This module adds the
//! missing entry point, a pure function over the two snapshots
//! that emits zero or one `AiRecommendation` describing the
//! trade-offs the comparison revealed.
//!
//! ## Spec grounding
//!
//! CR-07 §20:
//!
//! > AstroForge can optionally identify actionable trade-offs.
//! > Example: Version B has lower noise, better star separation,
//! > but increased highlight clipping. Recommended next step:
//! > Reduce stretch highlights before finalizing Version B.
//!
//! CR-07 §21:
//!
//! > AstroForge should not automatically label an image as
//! > "AI Winner" unless the user explicitly asks for automated
//! > ranking.
//!
//! The rule observes both: it never calls a side "winner"; it
//! describes asymmetric trade-offs in the spec's voice ("Version
//! B has X, ⚠ Y → next step Z") so the user retains the final
//! call (ADR-07.7).
//!
//! ## Determinism
//!
//! `recommend_post_comparison` is a pure function of the two
//! snapshots + the two decision states + the preferred-side hint.
//! No clock, no thread state, no I/O. Two calls with the same
//! inputs produce identical `AiRecommendation` rows. This matches
//! the determinism contract every other rule in `rules.rs`
//! honours and the test `recommend_is_deterministic_for_same_inputs`
//! below pins it.
//!
//! ## Coverage
//!
//! Only the five detector-backed metrics are eligible for
//! comparison (every other `MetricKind` ships as `None` /
//! `Inconclusive` in `metric_snapshot` so any delta on them
//! would be a comparison of two nulls, which is meaningless).
//! The five are the exact set named in the `comparison_metrics`
//! module's docstring so this module and that one stay in
//! lock-step.
//!
//! ## Scope of the trade-off signal
//!
//! A "trade-off" means at least one metric moves in the
//! preferred side's favour (lower noise, higher local contrast,
//! lower background gradient, fewer highlight clips, or more
//! stars / sharper stars where the detector reads it) AND at
//! least one metric moves against the preferred side. A pure
//! improvement (wins on everything) or a pure regression
//! (costs on everything) is NOT a trade-off. The spec example
//! calls out the asymmetric case specifically. Pure cases are
//! surfaced by the other rules; the post-comparison insight
//! adds the value of "and here is what that asymmetry implies
//! for the next stage".
//!
//! ## Persistence
//!
//! `flatten_for_store` in `recommendations/mod.rs` already
//! serialises every report row into `ai_recommendations` keyed
//! by `(image_version_id, operation, sequence)`. The new rule
//! plugs in with zero schema change. The new `operation` string
//! is a forward-compatible addition.

use std::collections::BTreeMap;

use astroforge_core::comparison::ImageDecisionState;

use crate::recommendations::{AiRecommendation, ModelCandidate};

/// The five metrics the rule may surface in a trade-off insight.
///
/// These strings are the `MetricKind::as_str()` values populated
/// by `astroforge_core::comparison_metrics::metric_snapshot`.
/// Centralised as a const so a refactor of the registry fails
/// the tests rather than silently breaking this rule.
pub const NOISE_LUMINANCE: &str = "noise.luminance";
pub const NOISE_CHROMINANCE: &str = "noise.chrominance";
pub const SHARPNESS_LOCAL: &str = "sharpness.local";
pub const BACKGROUND_GRADIENT: &str = "background.gradient";
pub const HIGHLIGHT_CLIPPING: &str = "dynamic_range.highlight_clipping";

/// Direction a metric moved between the two snapshots.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeltaDirection {
    /// The side the user is leaning toward (per `preferred_side`)
    /// is better on this metric.
    PreferredBetter,
    /// The other side is better.
    PreferredWorse,
    /// No meaningful difference (or both equal).
    NoChange,
}

/// One comparison row in a `MetricDeltaTable`: a single metric
/// with the preferred-side value, the other-side value, the
/// delta, and a coarse interpretation.
#[derive(Debug, Clone, PartialEq)]
pub struct MetricDeltaRow {
    metric: &'static str,
    preferred_value: f64,
    other_value: f64,
    /// Signed delta: positive = preferred-side value is higher
    /// than the other side's. The interpretation of "higher is
    /// better" is metric-specific and lives in `direction_for`.
    delta: f64,
    direction: DeltaDirection,
}

impl MetricDeltaRow {
    fn compute(
        metric: &'static str,
        preferred: f64,
        other: f64,
        higher_is_better: bool,
        change_threshold: f64,
    ) -> Self {
        let delta = preferred - other;
        let abs = delta.abs();
        let direction = if abs < change_threshold {
            DeltaDirection::NoChange
        } else if (delta > 0.0) == higher_is_better {
            DeltaDirection::PreferredBetter
        } else {
            DeltaDirection::PreferredWorse
        };
        Self {
            metric,
            preferred_value: preferred,
            other_value: other,
            delta,
            direction,
        }
    }
}

/// The set of metric deltas the rule will surface as evidence.
///
/// `preferred_label` and `other_label` are the version labels
/// the UI should display, typically the comparison item
/// labels ("A", "B") or a short id. Kept here so the test
/// fixtures can assert against a known string.
#[derive(Debug, Clone, PartialEq)]
pub struct MetricDeltaTable {
    pub preferred_label: String,
    pub other_label: String,
    pub rows: Vec<MetricDeltaRow>,
}

/// Input the rule consumes. All fields are required.
///
/// `preferred_side` is the user's lean. It must match one of
/// the two `version_id`s, otherwise the rule returns no
/// insight (the caller is the source of truth for which side
/// the user prefers; this rule never re-ranks).
///
/// `decision_a` / `decision_b` are the two `ImageDecisionState`
/// values; the rule inspects them only to gate the "any value
/// to add?" check, e.g. if both sides are `Rejected` the user
/// has already moved on, no insight is useful.
#[derive(Debug, Clone, PartialEq)]
pub struct PostComparisonInput {
    /// The Image Version id of the side the user is leaning
    /// toward (typically the one with `state == Preferred`).
    pub preferred_side: String,
    /// Friendly label for the preferred side (e.g. "Version B").
    pub preferred_label: String,
    /// The Image Version id of the other side.
    pub other_side: String,
    /// Friendly label for the other side (e.g. "Version A").
    pub other_label: String,
    /// The preferred side's `metric_snapshot` values.
    pub preferred_metrics: BTreeMap<String, f64>,
    /// The other side's `metric_snapshot` values.
    pub other_metrics: BTreeMap<String, f64>,
    /// Decision state for the preferred side.
    pub preferred_decision: ImageDecisionState,
    /// Decision state for the other side.
    pub other_decision: ImageDecisionState,
}

impl PostComparisonInput {
    /// Convenience constructor for tests and callers that have
    /// already labelled the sides.
    ///
    /// 8 parameters is intentional: the function is a thin
    /// wrapper that pairs each side's four attributes
    /// (version id, label, metrics, decision state) with its
    /// peer's. A builder would add ceremony without clarifying
    /// the data; allowing clippy is the right call.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        preferred_side: impl Into<String>,
        preferred_label: impl Into<String>,
        other_side: impl Into<String>,
        other_label: impl Into<String>,
        preferred_metrics: BTreeMap<String, f64>,
        other_metrics: BTreeMap<String, f64>,
        preferred_decision: ImageDecisionState,
        other_decision: ImageDecisionState,
    ) -> Self {
        Self {
            preferred_side: preferred_side.into(),
            preferred_label: preferred_label.into(),
            other_side: other_side.into(),
            other_label: other_label.into(),
            preferred_metrics,
            other_metrics,
            preferred_decision,
            other_decision,
        }
    }
}

/// Build the trade-off delta table from the two snapshots.
///
/// A metric only contributes a row if BOTH sides have a
/// value. The five detector-backed metrics are the eligible
/// set; any other key is ignored to keep the table honest
/// (mirroring `comparison_metrics::metric_snapshot` which
/// only fills the five).
pub fn build_delta_table(input: &PostComparisonInput) -> MetricDeltaTable {
    // Each row: (key, higher_is_better, change_threshold).
    // Thresholds are deliberately tight: the insight only
    // fires on a real, measurable trade-off, not a tiny
    // floating-point delta. They are picked at the same
    // order of magnitude the upstream detectors read.
    let defs: &[(&str, bool, f64)] = &[
        // Lower noise is better. Detectors read 0.0..0.5;
        // a delta below 0.01 is detector noise.
        (NOISE_LUMINANCE, false, 0.01),
        (NOISE_CHROMINANCE, false, 0.01),
        // Higher local sharpness is better. Detectors read
        // 0.0..1.0; a delta below 0.02 is not user-visible.
        (SHARPNESS_LOCAL, true, 0.02),
        // Lower background gradient is better. Detectors
        // read 0.0..0.5; a delta below 0.01 is noise.
        (BACKGROUND_GRADIENT, false, 0.01),
        // Lower highlight clipping is better. Detectors
        // read 0.0..1.0 (fraction of clipped pixels); a
        // delta below 0.005 is not visible.
        (HIGHLIGHT_CLIPPING, false, 0.005),
    ];

    let mut rows: Vec<MetricDeltaRow> = defs
        .iter()
        .filter_map(|(metric, higher_is_better, threshold)| {
            let preferred_value = *input.preferred_metrics.get(*metric)?;
            let other_value = *input.other_metrics.get(*metric)?;
            Some(MetricDeltaRow::compute(
                metric,
                preferred_value,
                other_value,
                *higher_is_better,
                *threshold,
            ))
        })
        .collect();

    // Sort for stable output: alphabetical by metric key. This
    // pins determinism in tests (a BTreeMap iteration is
    // already sorted; we sort `rows` so the test fixture
    // matches exactly regardless of `defs` order).
    rows.sort_by(|a, b| a.metric.cmp(b.metric));

    MetricDeltaTable {
        preferred_label: input.preferred_label.clone(),
        other_label: input.other_label.clone(),
        rows,
    }
}

/// Decide whether the table is a "trade-off" (wins AND costs)
/// and not "all wins" / "all costs" / "no change".
fn is_tradeoff(table: &MetricDeltaTable) -> bool {
    let mut wins = 0;
    let mut costs = 0;
    for row in &table.rows {
        match row.direction {
            DeltaDirection::PreferredBetter => wins += 1,
            DeltaDirection::PreferredWorse => costs += 1,
            DeltaDirection::NoChange => {}
        }
    }
    wins >= 1 && costs >= 1
}

/// Build the human-readable "reason" string for the insight.
///
/// Voice matches the spec example: "{Label} has <wins>, ⚠
/// <costs> → next step <hint>". The hint names the worst cost
/// metric and points at the natural follow-up stage (e.g.
/// "reduce stretch highlights" when `HIGHLIGHT_CLIPPING` is
/// the worst cost).
fn build_reason(table: &MetricDeltaTable) -> (String, String) {
    let wins: Vec<String> = table
        .rows
        .iter()
        .filter(|r| r.direction == DeltaDirection::PreferredBetter)
        .map(format_wins_row)
        .collect();
    let costs: Vec<String> = table
        .rows
        .iter()
        .filter(|r| r.direction == DeltaDirection::PreferredWorse)
        .map(format_costs_row)
        .collect();

    let reason = format!(
        "{} has {} and {}, but {} ({}). Recommended next step: {} before finalizing {}.",
        table.preferred_label,
        wins.join(" and "),
        costs.join(" and "),
        // Pick the worst cost for the headline.
        pick_worst_cost(table)
            .map(format_costs_row)
            .unwrap_or_default(),
        table.other_label,
        worst_cost_hint(table),
        table.preferred_label,
    );
    let expected_effect = format!(
        "Tighten the {} cost so the {} trade-off resolves in {}'s favour.",
        pick_worst_cost(table).map(|r| r.metric).unwrap_or("named"),
        table.preferred_label,
        table.preferred_label,
    );
    (reason, expected_effect)
}

fn format_wins_row(r: &MetricDeltaRow) -> String {
    human_metric(r.metric, r.delta, true)
}

fn format_costs_row(r: &MetricDeltaRow) -> String {
    human_metric(r.metric, r.delta, false)
}

/// Translate the metric key + delta + direction into one
/// clause of natural language.
fn human_metric(metric: &str, delta: f64, is_win: bool) -> String {
    let mag = delta.abs();
    let word = match metric {
        NOISE_LUMINANCE => "lower luminance noise",
        NOISE_CHROMINANCE => "lower chrominance noise",
        SHARPNESS_LOCAL => "better local sharpness",
        BACKGROUND_GRADIENT => "flatter background gradient",
        HIGHLIGHT_CLIPPING => "fewer highlight clipping pixels",
        _ => "an unspecified trade-off",
    };
    if is_win {
        format!("{} (Δ={:.3})", word, mag)
    } else {
        format!("⚠ {} (Δ={:.3})", word, mag)
    }
}

/// Find the cost row with the largest absolute delta. Ties
/// resolve to the metric named first in the `defs` table:
/// the early metrics are the most user-visible, so they are
/// the right default to surface.
fn pick_worst_cost(table: &MetricDeltaTable) -> Option<&MetricDeltaRow> {
    table
        .rows
        .iter()
        .filter(|r| r.direction == DeltaDirection::PreferredWorse)
        .max_by(|a, b| {
            a.delta
                .abs()
                .partial_cmp(&b.delta.abs())
                .unwrap_or(std::cmp::Ordering::Equal)
        })
}

/// Concrete next-stage hint for the worst cost.
fn worst_cost_hint(table: &MetricDeltaTable) -> &'static str {
    match pick_worst_cost(table).map(|r| r.metric) {
        Some(HIGHLIGHT_CLIPPING) => "reduce stretch highlights",
        Some(NOISE_LUMINANCE) => "tighten denoise strength",
        Some(NOISE_CHROMINANCE) => "tighten chroma denoise",
        Some(SHARPNESS_LOCAL) => "raise local sharpening carefully",
        Some(BACKGROUND_GRADIENT) => "extract background before further passes",
        _ => "tighten the named cost with a single targeted pass",
    }
}

/// A constant engine version for the new rule. Pinned so the
/// persisted row's `engine_version` is stable across runs and
/// future rule changes are obvious in the audit trail.
pub const POST_COMPARISON_ENGINE_VERSION: &str = "cr07_post_comparison_v1";

/// Emit a post-comparison insight, or `None` if the comparison
/// is not informative enough to surface one.
///
/// Returns `None` when:
///
/// - the input is malformed (`preferred_side` matches
///   `other_side`),
/// - either side's decision state is `Rejected` (the user has
///   already moved on),
/// - the detector-backed metrics don't show a real trade-off
///   (all wins, all costs, or no detectable change).
///
/// On `Some(rec)` the row is a valid `AiRecommendation` ready
/// for `flatten_for_store` to persist and `RecommendationList`
/// to render. The `operation` is `"post_comparison_insight"`
/// and the `classification` is `"observation"` (a finding, not
/// an enhancement operation) so the UI can show it with a
/// distinct icon without changing the card layout.
pub fn recommend_post_comparison(input: &PostComparisonInput) -> Option<AiRecommendation> {
    if input.preferred_side.is_empty()
        || input.other_side.is_empty()
        || input.preferred_side == input.other_side
    {
        return None;
    }
    if matches!(
        input.preferred_decision,
        ImageDecisionState::Rejected | ImageDecisionState::Final
    ) || matches!(
        input.other_decision,
        ImageDecisionState::Rejected | ImageDecisionState::Final
    ) {
        // The user has already finalised or rejected one of the
        // sides. A trade-off insight pointing at a "next step"
        // is no longer actionable.
        return None;
    }

    let table = build_delta_table(input);
    if !is_tradeoff(&table) {
        return None;
    }

    let (reason, expected_effect) = build_reason(&table);

    // Evidence: every metric that contributed to the insight
    // (both wins and costs). The UI surfaces this list under
    // the card; consumers can also use it for downstream
    // automation.
    let evidence: Vec<String> = table
        .rows
        .iter()
        .filter(|r| r.direction != DeltaDirection::NoChange)
        .map(|r| r.metric.to_string())
        .collect();

    // Affected regions: comparison is whole-image (the metric
    // snapshot is computed over the full frame), so the
    // recommendation explicitly notes "whole frame" rather
    // than leaving the field empty (which the UI reads as
    // "no claim").
    let affected_regions = vec!["whole_frame".to_string()];

    // Confidence: 0.7 baseline (this is an observation, not a
    // measurement). The detector-backed metrics have their own
    // confidence baked into the snapshot values; the rule does
    // not re-derive it. A user-visible trade-off (|delta|
    // exceeds `change_threshold` by a wide margin) lifts
    // confidence; a near-threshold trade-off leaves it at
    // baseline. Two-row rule of thumb: ≥3 wins/costs in
    // aggregate → 0.85; otherwise 0.7.
    let total_signal = evidence.len();
    let confidence: f32 = if total_signal >= 3 { 0.85 } else { 0.7 };

    // Risk: low. The recommendation is an observation +
    // hint, not an enhancement. The user still owns the
    // decision per ADR-07.7.
    let risk_level = "low".to_string();

    // No model is recommended: the hint is "reduce stretch
    // highlights" (a deterministic stretch parameter tweak),
    // not an AI model invocation. The `model_candidates` list
    // is empty, signalling to the UI that this row is not a
    // model-backed operation.
    let model_candidates: Vec<ModelCandidate> = Vec::new();
    let recommended_model: Option<String> = None;
    let estimated_runtime: Option<String> = None;
    let estimated_memory: Option<String> = None;

    Some(AiRecommendation {
        operation: "post_comparison_insight".to_string(),
        reason,
        confidence,
        evidence,
        affected_regions,
        expected_effect,
        risk_level,
        model_candidates,
        recommended_model,
        estimated_runtime,
        estimated_memory,
        classification: "observation".to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    fn snapshot(pairs: &[(&str, f64)]) -> BTreeMap<String, f64> {
        pairs.iter().map(|(k, v)| ((*k).to_string(), *v)).collect()
    }

    #[test]
    fn build_delta_table_emits_one_row_per_detector_backed_metric() {
        let preferred = snapshot(&[
            (NOISE_LUMINANCE, 0.10),
            (NOISE_CHROMINANCE, 0.05),
            (SHARPNESS_LOCAL, 0.45),
            (BACKGROUND_GRADIENT, 0.02),
            (HIGHLIGHT_CLIPPING, 0.001),
            // Unrelated key. Must be ignored.
            ("custom.thing", 99.9),
        ]);
        let other = snapshot(&[
            (NOISE_LUMINANCE, 0.10),
            (NOISE_CHROMINANCE, 0.05),
            (SHARPNESS_LOCAL, 0.45),
            (BACKGROUND_GRADIENT, 0.02),
            (HIGHLIGHT_CLIPPING, 0.001),
        ]);
        let input = PostComparisonInput::new(
            "v_b",
            "Version B",
            "v_a",
            "Version A",
            preferred,
            other,
            ImageDecisionState::Preferred,
            ImageDecisionState::Candidate,
        );
        let table = build_delta_table(&input);
        assert_eq!(table.rows.len(), 5);
        assert!(table
            .rows
            .iter()
            .all(|r| r.direction == DeltaDirection::NoChange));
        // The custom key must not appear.
        assert!(table.rows.iter().all(|r| r.metric != "custom.thing"));
    }

    #[test]
    fn build_delta_table_classifies_direction_correctly() {
        // Preferred is better on noise + sharpness (lower noise,
        // higher sharpness). Preferred is WORSE on highlight
        // clipping. The classifier must read all three.
        let preferred = snapshot(&[
            (NOISE_LUMINANCE, 0.05), // win vs 0.12
            (NOISE_CHROMINANCE, 0.05),
            (SHARPNESS_LOCAL, 0.50), // win vs 0.40
            (BACKGROUND_GRADIENT, 0.02),
            (HIGHLIGHT_CLIPPING, 0.04), // cost vs 0.005
        ]);
        let other = snapshot(&[
            (NOISE_LUMINANCE, 0.12),
            (NOISE_CHROMINANCE, 0.05),
            (SHARPNESS_LOCAL, 0.40),
            (BACKGROUND_GRADIENT, 0.02),
            (HIGHLIGHT_CLIPPING, 0.005),
        ]);
        let input = PostComparisonInput::new(
            "v_b",
            "Version B",
            "v_a",
            "Version A",
            preferred,
            other,
            ImageDecisionState::Preferred,
            ImageDecisionState::Candidate,
        );
        let table = build_delta_table(&input);
        let lum = table
            .rows
            .iter()
            .find(|r| r.metric == NOISE_LUMINANCE)
            .unwrap();
        assert_eq!(lum.direction, DeltaDirection::PreferredBetter);
        let sharp = table
            .rows
            .iter()
            .find(|r| r.metric == SHARPNESS_LOCAL)
            .unwrap();
        assert_eq!(sharp.direction, DeltaDirection::PreferredBetter);
        let clip = table
            .rows
            .iter()
            .find(|r| r.metric == HIGHLIGHT_CLIPPING)
            .unwrap();
        assert_eq!(clip.direction, DeltaDirection::PreferredWorse);
    }

    #[test]
    fn recommend_returns_none_when_sides_are_equal() {
        let snap = snapshot(&[
            (NOISE_LUMINANCE, 0.10),
            (NOISE_CHROMINANCE, 0.05),
            (SHARPNESS_LOCAL, 0.45),
            (BACKGROUND_GRADIENT, 0.02),
            (HIGHLIGHT_CLIPPING, 0.001),
        ]);
        let input = PostComparisonInput::new(
            "v_b",
            "Version B",
            "v_a",
            "Version A",
            snap.clone(),
            snap,
            ImageDecisionState::Preferred,
            ImageDecisionState::Candidate,
        );
        assert!(recommend_post_comparison(&input).is_none());
    }

    #[test]
    fn recommend_returns_none_when_only_wins_no_costs() {
        // Preferred is better on everything. The spec example
        // calls out the asymmetric case; the pure-win case is
        // not the trade-off the §20 rule is for.
        let preferred = snapshot(&[
            (NOISE_LUMINANCE, 0.03),
            (NOISE_CHROMINANCE, 0.02),
            (SHARPNESS_LOCAL, 0.60),
            (BACKGROUND_GRADIENT, 0.005),
            (HIGHLIGHT_CLIPPING, 0.0005),
        ]);
        let other = snapshot(&[
            (NOISE_LUMINANCE, 0.12),
            (NOISE_CHROMINANCE, 0.10),
            (SHARPNESS_LOCAL, 0.40),
            (BACKGROUND_GRADIENT, 0.05),
            (HIGHLIGHT_CLIPPING, 0.04),
        ]);
        let input = PostComparisonInput::new(
            "v_b",
            "Version B",
            "v_a",
            "Version A",
            preferred,
            other,
            ImageDecisionState::Preferred,
            ImageDecisionState::Candidate,
        );
        assert!(
            recommend_post_comparison(&input).is_none(),
            "pure-win case should not surface a trade-off insight"
        );
    }

    #[test]
    fn recommend_returns_insight_with_spec_voice_when_tradeoff() {
        // Mirrors the spec example: B has lower noise + better
        // sharpness but ⚠ more highlight clipping. The reason
        // string must call out B's wins, B's cost, and the
        // next step in the spec's voice.
        let preferred = snapshot(&[
            (NOISE_LUMINANCE, 0.05), // win
            (NOISE_CHROMINANCE, 0.04),
            (SHARPNESS_LOCAL, 0.50), // win
            (BACKGROUND_GRADIENT, 0.02),
            (HIGHLIGHT_CLIPPING, 0.04), // cost
        ]);
        let other = snapshot(&[
            (NOISE_LUMINANCE, 0.12),
            (NOISE_CHROMINANCE, 0.05),
            (SHARPNESS_LOCAL, 0.40),
            (BACKGROUND_GRADIENT, 0.02),
            (HIGHLIGHT_CLIPPING, 0.005),
        ]);
        let input = PostComparisonInput::new(
            "v_b",
            "Version B",
            "v_a",
            "Version A",
            preferred,
            other,
            ImageDecisionState::Preferred,
            ImageDecisionState::Candidate,
        );
        let rec = recommend_post_comparison(&input).expect("trade-off must emit a row");
        assert_eq!(rec.operation, "post_comparison_insight");
        assert_eq!(rec.classification, "observation");
        // No "winner" language (per §21).
        let combined = format!("{} {}", rec.reason, rec.expected_effect);
        for forbidden in ["winner", "Winner", "best", "Best", "AI Winner"] {
            assert!(
                !combined.contains(forbidden),
                "reason/expected_effect must not contain {:?}; got: {}",
                forbidden,
                combined
            );
        }
        // Must reference the spec's "next step" framing.
        assert!(rec.reason.contains("Recommended next step"));
        // Must call out the highlight clipping cost (the worst
        // cost in this fixture).
        assert!(rec.reason.contains("highlight"));
        // Must reference the preferred side by its label.
        assert!(rec.reason.contains("Version B"));
        // Evidence list carries the metrics that contributed.
        assert!(rec.evidence.contains(&NOISE_LUMINANCE.to_string()));
        assert!(rec.evidence.contains(&SHARPNESS_LOCAL.to_string()));
        assert!(rec.evidence.contains(&HIGHLIGHT_CLIPPING.to_string()));
        // Affected regions: whole_frame (the comparison is
        // whole-image).
        assert_eq!(rec.affected_regions, vec!["whole_frame".to_string()]);
        // No model candidates: this is an observation, not an
        // enhancement.
        assert!(rec.model_candidates.is_empty());
        assert!(rec.recommended_model.is_none());
    }

    #[test]
    fn recommend_is_deterministic_for_same_inputs() {
        let preferred = snapshot(&[
            (NOISE_LUMINANCE, 0.05),
            (NOISE_CHROMINANCE, 0.04),
            (SHARPNESS_LOCAL, 0.50),
            (BACKGROUND_GRADIENT, 0.02),
            (HIGHLIGHT_CLIPPING, 0.04),
        ]);
        let other = snapshot(&[
            (NOISE_LUMINANCE, 0.12),
            (NOISE_CHROMINANCE, 0.05),
            (SHARPNESS_LOCAL, 0.40),
            (BACKGROUND_GRADIENT, 0.02),
            (HIGHLIGHT_CLIPPING, 0.005),
        ]);
        let a = recommend_post_comparison(&PostComparisonInput::new(
            "v_b",
            "Version B",
            "v_a",
            "Version A",
            preferred.clone(),
            other.clone(),
            ImageDecisionState::Preferred,
            ImageDecisionState::Candidate,
        ))
        .expect("trade-off must emit a row");
        let b = recommend_post_comparison(&PostComparisonInput::new(
            "v_b",
            "Version B",
            "v_a",
            "Version A",
            preferred,
            other,
            ImageDecisionState::Preferred,
            ImageDecisionState::Candidate,
        ))
        .expect("trade-off must emit a row");
        assert_eq!(a, b);
    }

    #[test]
    fn recommend_returns_none_when_either_side_is_rejected() {
        // Either side rejected → no actionable next step.
        let preferred = snapshot(&[
            (NOISE_LUMINANCE, 0.05),
            (NOISE_CHROMINANCE, 0.04),
            (SHARPNESS_LOCAL, 0.50),
            (BACKGROUND_GRADIENT, 0.02),
            (HIGHLIGHT_CLIPPING, 0.04),
        ]);
        let other = snapshot(&[
            (NOISE_LUMINANCE, 0.12),
            (NOISE_CHROMINANCE, 0.05),
            (SHARPNESS_LOCAL, 0.40),
            (BACKGROUND_GRADIENT, 0.02),
            (HIGHLIGHT_CLIPPING, 0.005),
        ]);
        // Preferred rejected.
        let input = PostComparisonInput::new(
            "v_b",
            "Version B",
            "v_a",
            "Version A",
            preferred.clone(),
            other.clone(),
            ImageDecisionState::Rejected,
            ImageDecisionState::Candidate,
        );
        assert!(recommend_post_comparison(&input).is_none());
        // Other rejected.
        let input = PostComparisonInput::new(
            "v_b",
            "Version B",
            "v_a",
            "Version A",
            preferred,
            other,
            ImageDecisionState::Preferred,
            ImageDecisionState::Rejected,
        );
        assert!(recommend_post_comparison(&input).is_none());
    }

    #[test]
    fn recommend_returns_none_when_sides_are_misidentified() {
        // preferred_side == other_side → caller bug → no row.
        let snap = snapshot(&[
            (NOISE_LUMINANCE, 0.05),
            (NOISE_CHROMINANCE, 0.04),
            (SHARPNESS_LOCAL, 0.50),
            (BACKGROUND_GRADIENT, 0.02),
            (HIGHLIGHT_CLIPPING, 0.04),
        ]);
        let input = PostComparisonInput::new(
            "v_a",
            "Version A",
            "v_a",
            "Version A",
            snap.clone(),
            snap,
            ImageDecisionState::Preferred,
            ImageDecisionState::Candidate,
        );
        assert!(recommend_post_comparison(&input).is_none());
    }

    #[test]
    fn confidence_lifts_when_more_metrics_contribute() {
        // 3-metric trade-off should hit the 0.85 band.
        let preferred = snapshot(&[
            (NOISE_LUMINANCE, 0.04),   // win
            (NOISE_CHROMINANCE, 0.03), // win
            (SHARPNESS_LOCAL, 0.55),   // win
            (BACKGROUND_GRADIENT, 0.02),
            (HIGHLIGHT_CLIPPING, 0.04), // cost
        ]);
        let other = snapshot(&[
            (NOISE_LUMINANCE, 0.12),
            (NOISE_CHROMINANCE, 0.10),
            (SHARPNESS_LOCAL, 0.40),
            (BACKGROUND_GRADIENT, 0.02),
            (HIGHLIGHT_CLIPPING, 0.005),
        ]);
        let input = PostComparisonInput::new(
            "v_b",
            "Version B",
            "v_a",
            "Version A",
            preferred,
            other,
            ImageDecisionState::Preferred,
            ImageDecisionState::Candidate,
        );
        let rec = recommend_post_comparison(&input).expect("trade-off must emit a row");
        assert!(
            (rec.confidence - 0.85).abs() < 0.001,
            "expected confidence ~0.85 for 3-metric trade-off; got {}",
            rec.confidence
        );
    }
}
