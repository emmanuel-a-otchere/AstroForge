//! CR-05 P3 slice 2 — recommendation engine (per CR-05 §14, §27, §31).
//!
//! Reads `QualityMetricSnapshot` rows from prior stage executions
//! and emits a deterministic `Recommendation` per rule. The
//! frontend calls `get_recommendation(stage_execution_id)` to
//! surface these.
//!
//! Slice 2 ships three rules:
//! - `stretch_recommendation`: picks `shadows` / `highlights` /
//!   `midtones` for the next stretch stage based on the image's
//!   mean / stddev / star count.
//! - `denoise_recommendation`: picks `dip_amount` /
//!   `dip_iterations` / `blend_ratio` based on SNR.
//! - `background_recommendation`: picks `method` /
//!   `sample_fraction` based on the background gradient.
//!
//! All rules are pure functions of the input snapshot (no clock /
//! thread state). Two calls with the same inputs produce identical
//! outputs — necessary for deterministic preview re-runs (P4).

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::domain::StageExecution;
#[cfg(test)]
use crate::pipeline_plan::StageType;
use crate::quality::QualityMetricSnapshot;

// ─── Domain types ────────────────────────────────────────────────────────

/// CR-05 P3 slice 2 — a single parameter override keyed by the
/// stage parameter name (e.g. "shadows", "dip_amount",
/// "sample_fraction"). Stored as `BTreeMap` so JSON output is
/// deterministically ordered.
pub type StageParameters = BTreeMap<String, serde_json::Value>;

/// CR-05 P3 slice 2 — a fully-derived recommendation for the next
/// stage in a plan. Carries the recommended parameters, the
/// rationale, and the source rule id.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProcessingDecision {
    /// Target stage type, e.g. "stretch", "denoise", "background".
    pub stage_type: String,
    /// Recommended parameters for that stage. Empty when no
    /// parameters need to change.
    pub parameters: StageParameters,
    /// One-line human-readable rationale, e.g.
    /// "low stddev (0.04) suggests an aggressive stretch is safe".
    pub rationale: String,
}

/// CR-05 P3 slice 2 — a recommendation persisted against a stage
/// execution. One row per rule that fires for a given snapshot.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Recommendation {
    pub id: String,
    pub stage_execution_id: String,
    /// Identifier of the rule that produced this recommendation.
    /// Stable across runs (per CR-05 §14).
    pub rule_id: String,
    pub decision: ProcessingDecision,
    /// 0.0..=1.0 — how confident the rule is. Used by the
    /// frontend to order recommendations and by the recommendation
    /// store to surface the top picks per stage. Slice 2 ships
    /// heuristic confidence (rule-level, not statistical).
    pub confidence: f64,
    /// One-line evidence summary, e.g.
    /// "mean=0.10, stddev=0.04, star_count=12".
    pub evidence_summary: String,
    /// `unix_ms:<ms>` per CR-05 §15 (no calendar dates in Rust).
    pub created_at: String,
}

// ─── Rule trait ──────────────────────────────────────────────────────────

/// CR-05 P3 slice 2 — a deterministic recommendation rule. All
/// rules read the prior `QualityMetricSnapshot` and return an
/// optional `ProcessingDecision`. `None` means the rule does not
/// apply (the rule is then omitted from `get_recommendation`'s
/// output for that stage execution).
pub trait RecommendationRule: Send + Sync {
    /// Stable identifier, e.g. "stretch_v1".
    fn rule_id(&self) -> &'static str;

    /// Which stage type this rule targets. A single rule targets
    /// exactly one stage type.
    fn target_stage_type(&self) -> &'static str;

    /// Compute a recommendation from a snapshot, or return `None`
    /// if the rule doesn't apply. Must be pure: no I/O, no clock.
    fn evaluate(&self, snapshot: &QualityMetricSnapshot) -> Option<ProcessingDecision>;
}

// ─── Built-in rules ──────────────────────────────────────────────────────

/// CR-05 P3 slice 2 — stretch parameter recommendation.
///
/// Heuristic: pick `shadows` / `highlights` based on the image's
/// mean and stddev; pick `midtones` based on the star count (more
/// stars = preserve more midtone balance).
pub struct StretchRecommendation;

impl RecommendationRule for StretchRecommendation {
    fn rule_id(&self) -> &'static str {
        "stretch_v1"
    }

    fn target_stage_type(&self) -> &'static str {
        "stretch"
    }

    fn evaluate(&self, snapshot: &QualityMetricSnapshot) -> Option<ProcessingDecision> {
        // Skipped for uniform images (no signal).
        if snapshot.stddev < 1e-6 {
            return None;
        }

        // Aggressive stretch when the mean is high and stddev is
        // low (background-dominated). Conservative stretch when
        // mean is low or stddev is high (signal already fills the
        // dynamic range).
        let dynamic_range = snapshot.mean + 3.0 * snapshot.stddev;
        let shadows = if dynamic_range < 0.3 {
            0.05
        } else if dynamic_range > 0.7 {
            0.20
        } else {
            0.10
        };
        let highlights = if snapshot.stddev > 0.30 { 0.85 } else { 0.95 };

        // More stars => keep more midtone balance; few stars =>
        // push midtones for contrast.
        let midtones = if snapshot.star_count > 50 { 0.50 } else { 0.45 };

        let rationale = format!(
            "image mean={:.3} stddev={:.3} star_count={}: shadows={:.2} highlights={:.2} midtones={:.2}",
            snapshot.mean, snapshot.stddev, snapshot.star_count, shadows, highlights, midtones
        );

        let mut parameters = BTreeMap::new();
        parameters.insert("shadows".into(), serde_json::json!(shadows));
        parameters.insert("highlights".into(), serde_json::json!(highlights));
        parameters.insert("midtones".into(), serde_json::json!(midtones));

        Some(ProcessingDecision {
            stage_type: "stretch".into(),
            parameters,
            rationale,
        })
    }
}

/// CR-05 P3 slice 2 — denoise parameter recommendation.
///
/// Heuristic: higher SNR -> less denoising; lower SNR -> more
/// aggressive denoising with tighter blending.
pub struct DenoiseRecommendation;

impl RecommendationRule for DenoiseRecommendation {
    fn rule_id(&self) -> &'static str {
        "denoise_v1"
    }

    fn target_stage_type(&self) -> &'static str {
        "denoise"
    }

    fn evaluate(&self, snapshot: &QualityMetricSnapshot) -> Option<ProcessingDecision> {
        // SNR is undefined for uniform images; skip.
        if snapshot.stddev < 1e-6 {
            return None;
        }
        let snr = snapshot.snr_db;
        let (dip_amount, dip_iterations, blend_ratio) = if snr > 30.0 {
            (0.30, 2, 0.40)
        } else if snr > 15.0 {
            (0.50, 3, 0.60)
        } else {
            (0.70, 4, 0.80)
        };

        let rationale = format!(
            "snr_db={:.2}: dip_amount={:.2} dip_iterations={} blend_ratio={:.2}",
            snr, dip_amount, dip_iterations, blend_ratio
        );

        let mut parameters = BTreeMap::new();
        parameters.insert("dip_amount".into(), serde_json::json!(dip_amount));
        parameters.insert("dip_iterations".into(), serde_json::json!(dip_iterations));
        parameters.insert("blend_ratio".into(), serde_json::json!(blend_ratio));

        Some(ProcessingDecision {
            stage_type: "denoise".into(),
            parameters,
            rationale,
        })
    }
}

/// CR-05 P3 slice 2 — background parameter recommendation.
///
/// Heuristic: a strong background gradient favours polynomial
/// subtraction with a higher tolerance; a flat background uses
/// radial subtraction with a low tolerance.
pub struct BackgroundRecommendation;

impl RecommendationRule for BackgroundRecommendation {
    fn rule_id(&self) -> &'static str {
        "background_v1"
    }

    fn target_stage_type(&self) -> &'static str {
        "background"
    }

    fn evaluate(&self, snapshot: &QualityMetricSnapshot) -> Option<ProcessingDecision> {
        if snapshot.stddev < 1e-6 {
            return None;
        }
        let gradient = snapshot.background_gradient;
        let (method, sample_fraction, tolerance) = if gradient > 0.05 {
            ("polynomial", 0.20, 0.50)
        } else {
            ("radial", 0.10, 0.20)
        };

        let rationale = format!(
            "background_gradient={:.4}: method={} sample_fraction={:.2} tolerance={:.2}",
            gradient, method, sample_fraction, tolerance
        );

        let mut parameters = BTreeMap::new();
        parameters.insert("method".into(), serde_json::json!(method));
        parameters.insert("sample_fraction".into(), serde_json::json!(sample_fraction));
        parameters.insert("tolerance".into(), serde_json::json!(tolerance));

        Some(ProcessingDecision {
            stage_type: "background".into(),
            parameters,
            rationale,
        })
    }
}

/// CR-05 P3 slice 2 — default registry containing every built-in
/// rule. Slice 2 ships exactly three rules. Future slices register
/// more (e.g. "calibrate_v1" once the calibrate module exposes
/// data-driven tuning).
pub fn default_rules() -> Vec<Box<dyn RecommendationRule>> {
    vec![
        Box::new(StretchRecommendation),
        Box::new(DenoiseRecommendation),
        Box::new(BackgroundRecommendation),
    ]
}

// ─── Engine ──────────────────────────────────────────────────────────────

/// CR-05 P3 slice 2 — deterministic recommendation engine. Given
/// a `StageExecution`, look up its `metric_snapshot_json`, run
/// every applicable rule, and return a list of `Recommendation`
/// rows.
///
/// Rules that do not apply to the snapshot (return `None`) are
/// omitted. Recommendations are sorted by `rule_id` for stable
/// ordering across runs.
pub struct RecommendationEngine {
    rules: Vec<Box<dyn RecommendationRule>>,
}

impl RecommendationEngine {
    pub fn new(rules: Vec<Box<dyn RecommendationRule>>) -> Self {
        Self { rules }
    }

    /// CR-05 P3 slice 2 — convenience constructor using the
    /// default rule set.
    pub fn with_defaults() -> Self {
        Self::new(default_rules())
    }

    /// CR-05 P3 slice 2 — evaluate every rule against the
    /// snapshot on the given stage execution. Returns an empty
    /// Vec when the execution has no snapshot (no-op stages,
    /// metadata-only stages, failed stages).
    pub fn evaluate(
        &self,
        exec: &StageExecution,
        snapshot: &QualityMetricSnapshot,
    ) -> Vec<Recommendation> {
        let mut out: Vec<Recommendation> = self
            .rules
            .iter()
            .filter_map(|rule| {
                rule.evaluate(snapshot).map(|decision| Recommendation {
                    id: format!("rec_{}_{}", exec.stage_execution_id, rule.rule_id()),
                    stage_execution_id: exec.stage_execution_id.clone(),
                    rule_id: rule.rule_id().into(),
                    decision,
                    confidence: 0.75,
                    evidence_summary: evidence_summary(snapshot),
                    created_at: format!("unix_ms:{}", crate::pipeline_plan::plan::now_unix_ms()),
                })
            })
            .collect();
        // Deterministic ordering.
        out.sort_by(|a, b| a.rule_id.cmp(&b.rule_id));
        out
    }
}

/// CR-05 P3 slice 2 — one-line evidence summary shown to the
/// user in the IntelligencePanel. Stable format.
fn evidence_summary(snapshot: &QualityMetricSnapshot) -> String {
    format!(
        "mean={:.3} stddev={:.3} snr_db={:.2} fwhm={:.2} star_count={} bg_grad={:.4}",
        snapshot.mean,
        snapshot.stddev,
        snapshot.snr_db,
        snapshot.fwhm,
        snapshot.star_count,
        snapshot.background_gradient
    )
}

// ─── Tests ───────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::image::F32Image;

    fn synthetic_snapshot_with_stats(
        mean: f64,
        stddev: f64,
        star_count: u32,
        bg_grad: f64,
    ) -> QualityMetricSnapshot {
        QualityMetricSnapshot {
            width: 16,
            height: 16,
            channels: 1,
            mean,
            stddev,
            snr_db: if stddev > 1e-12 {
                20.0 * (mean.max(1e-12) / stddev).log10()
            } else {
                0.0
            },
            fwhm: 0.0,
            star_count,
            background_gradient: bg_grad,
        }
    }

    fn synthetic_exec(stage_type: StageType, status: &str) -> StageExecution {
        let label = stage_type.label().to_string();
        StageExecution {
            stage_execution_id: format!("exec_{}", label),
            plan_id: "plan_test".into(),
            stage_id: label,
            attempt: 1,
            status: status.into(),
            input_version_id: None,
            output_artifact_id: None,
            parameters_json: None,
            parameters_hash: None,
            started_at: Some("unix_ms:1".into()),
            completed_at: Some("unix_ms:2".into()),
            resource_usage_json: None,
            error_json: None,
            metric_snapshot_json: None,
        }
    }

    #[test]
    fn stretch_rule_picks_parameters_for_low_dynamic_range() {
        let snap = synthetic_snapshot_with_stats(0.05, 0.04, 5, 0.0);
        let rec = StretchRecommendation.evaluate(&snap).unwrap();
        assert_eq!(rec.stage_type, "stretch");
        let shadows = rec.parameters.get("shadows").unwrap().as_f64().unwrap();
        let highlights = rec.parameters.get("highlights").unwrap().as_f64().unwrap();
        let midtones = rec.parameters.get("midtones").unwrap().as_f64().unwrap();
        // mean + 3*stddev = 0.17 < 0.3 -> aggressive shadows
        assert!((shadows - 0.05).abs() < 1e-9);
        // stddev < 0.30 -> highlights stays at 0.95
        assert!((highlights - 0.95).abs() < 1e-9);
        // star_count <= 50 -> midtones pushed for contrast
        assert!((midtones - 0.45).abs() < 1e-9);
    }

    #[test]
    fn stretch_rule_skips_uniform_images() {
        let snap = synthetic_snapshot_with_stats(1.0, 0.0, 0, 0.0);
        assert!(StretchRecommendation.evaluate(&snap).is_none());
    }

    #[test]
    fn denoise_rule_scales_with_snr() {
        let high_snr = synthetic_snapshot_with_stats(0.50, 0.005, 100, 0.0);
        let low_snr = synthetic_snapshot_with_stats(0.50, 0.10, 100, 0.0);
        let hi = DenoiseRecommendation.evaluate(&high_snr).unwrap();
        let lo = DenoiseRecommendation.evaluate(&low_snr).unwrap();
        let hi_amount = hi.parameters.get("dip_amount").unwrap().as_f64().unwrap();
        let lo_amount = lo.parameters.get("dip_amount").unwrap().as_f64().unwrap();
        assert!(
            lo_amount > hi_amount,
            "low-snr image must demand more denoising"
        );
    }

    #[test]
    fn background_rule_picks_polynomial_for_strong_gradient() {
        let snap = synthetic_snapshot_with_stats(0.30, 0.10, 30, 0.10);
        let rec = BackgroundRecommendation.evaluate(&snap).unwrap();
        assert_eq!(
            rec.parameters.get("method").unwrap().as_str().unwrap(),
            "polynomial"
        );
    }

    #[test]
    fn background_rule_picks_radial_for_flat_background() {
        let snap = synthetic_snapshot_with_stats(0.30, 0.10, 30, 0.001);
        let rec = BackgroundRecommendation.evaluate(&snap).unwrap();
        assert_eq!(
            rec.parameters.get("method").unwrap().as_str().unwrap(),
            "radial"
        );
    }

    #[test]
    fn engine_evaluates_all_applicable_rules() {
        let snap = synthetic_snapshot_with_stats(0.10, 0.04, 12, 0.01);
        let exec = synthetic_exec(StageType::Stretch, "completed");
        let engine = RecommendationEngine::with_defaults();
        let recs = engine.evaluate(&exec, &snap);
        // 3 rules all apply to a non-uniform image with positive
        // stddev.
        assert_eq!(recs.len(), 3);
        // Sorted by rule_id (alphabetical).
        let rule_ids: Vec<&str> = recs.iter().map(|r| r.rule_id.as_str()).collect();
        let mut sorted = rule_ids.clone();
        sorted.sort();
        assert_eq!(rule_ids, sorted);
    }

    #[test]
    fn engine_omits_rules_that_do_not_apply() {
        let snap = synthetic_snapshot_with_stats(1.0, 0.0, 0, 0.0);
        let exec = synthetic_exec(StageType::Stretch, "completed");
        let engine = RecommendationEngine::with_defaults();
        let recs = engine.evaluate(&exec, &snap);
        // Uniform -> stretch / denoise / background all return None.
        assert_eq!(recs.len(), 0);
    }

    #[test]
    fn engine_is_deterministic_for_same_input() {
        let snap = synthetic_snapshot_with_stats(0.10, 0.04, 12, 0.01);
        let exec = synthetic_exec(StageType::Stretch, "completed");
        let engine = RecommendationEngine::with_defaults();
        let a = engine.evaluate(&exec, &snap);
        let b = engine.evaluate(&exec, &snap);
        // rule_id / parameters / rationale are all deterministic;
        // created_at differs by ms but is non-determinstic only in
        // the time field. We compare everything except created_at.
        for (ra, rb) in a.iter().zip(b.iter()) {
            assert_eq!(ra.rule_id, rb.rule_id);
            assert_eq!(ra.decision, rb.decision);
            assert_eq!(ra.evidence_summary, rb.evidence_summary);
            assert_eq!(ra.confidence, rb.confidence);
        }
    }

    #[test]
    fn recommendation_serialises_to_expected_json_keys() {
        let snap = synthetic_snapshot_with_stats(0.10, 0.04, 12, 0.01);
        let rec = StretchRecommendation.evaluate(&snap).unwrap();
        let json = serde_json::to_string(&rec).unwrap();
        assert!(json.contains("\"stage_type\":\"stretch\""));
        assert!(json.contains("\"parameters\""));
        assert!(json.contains("\"shadows\""));
        assert!(json.contains("\"rationale\""));
    }

    #[test]
    fn real_image_produces_three_recommendations() {
        // End-to-end with a real F32Image: compute_metrics then
        // engine.evaluate must produce 3 recommendations (stretch
        // / denoise / background) for any non-uniform image.
        let mut img = F32Image::new(8, 8, 1);
        for y in 0..8 {
            for x in 0..8 {
                img[(0, y, x)] = ((x + y) as f32) * 0.05;
            }
        }
        let snap = crate::quality::compute_metrics(&img);
        let exec = synthetic_exec(StageType::Stretch, "completed");
        let engine = RecommendationEngine::with_defaults();
        let recs = engine.evaluate(&exec, &snap);
        assert_eq!(recs.len(), 3);
        let stage_types: Vec<&str> = recs
            .iter()
            .map(|r| r.decision.stage_type.as_str())
            .collect();
        assert!(stage_types.contains(&"stretch"));
        assert!(stage_types.contains(&"denoise"));
        assert!(stage_types.contains(&"background"));
    }
}
