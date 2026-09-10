//! CR-06 P3 — AI Recommendation Engine.
//!
//! Translates an `ImageAnalysisReport` (P2) into a list of
//! `AiRecommendation` rows matching CR-06 §27's contract:
//!
//! ```text
//! Recommendation {
//!     operation, reason, confidence, evidence[], affected_regions[],
//!     expected_effect, risk_level, model_candidates[],
//!     recommended_model, estimated_runtime, estimated_memory,
//!     classification,
//! }
//! ```
//!
//! The pipeline is intentionally simple and inspectable —
//! every recommendation is the deterministic output of a
//! rule (`rules.rs`) given the analysis observations, and
//! the list is post-processed by `ordering.rs` to enforce
//! the §23 sequencing rules (denoise before detail, hot
//! pixels before denoise, super-resolution last, etc.).
//!
//! The engine never invents an operation the analysis does
//! not motivate: a recommendation is only emitted when an
//! observation crosses a documented threshold. This
//! preserves the trust boundary in CR-06 §3 — the system
//! explains its findings, never prescribes enhancements
//! the data does not justify.
//!
//! Resource estimates (`estimated_runtime`, `estimated_memory`)
//! are derived from the recommended model's tile config +
//! the image dimensions, not from a runtime measurement.
//! The estimates are honest projections that bound the
//! worst case; the user sees them before any work begins.

pub mod ordering;
pub mod resource_estimate;
pub mod rules;

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

pub use crate::recommendations::ordering::{order_recommendations, SequencingNote};
pub use crate::recommendations::resource_estimate::{estimate_for, ResourceEstimate};
pub use crate::recommendations::rules::{recommend, RecommendationSet};

/// One recommendation row, matching CR-06 §27 verbatim.
///
/// Persisted into the `ai_recommendations` table via
/// `DomainStore::upsert_ai_recommendation`. The
/// `core`-flattened fields (`operation`, `rationale`,
/// `confidence`, `risk_level`) live on the row directly;
/// the §27 payload fields (evidence, affected_regions,
/// expected_effect, model_candidates, recommended_model,
/// estimated_runtime, estimated_memory, classification)
/// are serialised into `payload_json`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AiRecommendation {
    pub operation: String,
    pub reason: String,
    pub confidence: f32,
    pub evidence: Vec<String>,
    pub affected_regions: Vec<String>,
    pub expected_effect: String,
    pub risk_level: String,
    pub model_candidates: Vec<ModelCandidate>,
    pub recommended_model: Option<String>,
    pub estimated_runtime: Option<String>,
    pub estimated_memory: Option<String>,
    pub classification: String,
}

/// One model that could carry out the operation.
///
/// `sha256` and `tile_size` come from the canonical
/// `astroforge-ai::hub::ModelInfo` registry (P3 reads the
/// registry to compute the resource estimates and pick
/// the recommended model).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModelCandidate {
    pub name: String,
    pub version: String,
    pub sha256: String,
    pub tile_size: u32,
}

/// The full recommendation set for an Image Version.
///
/// `recommendations` is the final ordered list (after
/// §23 sequencing). `sequencing_notes` records any
/// re-orderings the engine applied, plus the rationale
/// ("detail before denoise amplifies noise → reordered").
/// `image_version_id` and `project_id` are echoed from the
/// inputs so the caller can persist each row without
/// remembering which analysis produced them.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RecommendationReport {
    pub project_id: String,
    pub image_version_id: String,
    pub recommendations: Vec<AiRecommendation>,
    pub sequencing_notes: Vec<SequencingNote>,
    pub engine_version: String,
    pub created_at: String,
}

impl RecommendationReport {
    /// Serialize the report to a JSON string. The shape
    /// round-trips through `serde_json` so the caller can
    /// also use `from_json` to rebuild it.
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }

    pub fn from_json(raw: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(raw)
    }
}

/// Deterministic engine entry point.
///
/// Reads the analysis report, emits candidate
/// recommendations via `rules::recommend`, then post-processes
/// with `ordering::order_recommendations` to enforce §23
/// sequencing. The two steps are split so a future slice
/// can surface "sequencing notes" independently of the
/// recommendation list (CR-06 §23 lists them as a
/// separate UX surface).
///
/// The `now_iso` and `engine_version` parameters are
/// required (not read from `SystemTime` / `CARGO_PKG_VERSION`)
/// so a test can pin the report's `created_at` and
/// `engine_version` exactly. Production callers should use
/// `analyze` (the convenience wrapper) which fills both
/// from the live environment.
pub fn analyze_at(
    report: &astroforge_core::image_analysis::report::ImageAnalysisReport,
    project_id: &str,
    now_iso: &str,
    engine_version: &str,
) -> RecommendationReport {
    let rec_set = recommend(report);
    let (ordered, notes) = order_recommendations(rec_set.recommendations);
    RecommendationReport {
        project_id: project_id.to_string(),
        image_version_id: report.image_version_id.clone(),
        recommendations: ordered,
        sequencing_notes: notes,
        engine_version: engine_version.to_string(),
        created_at: now_iso.to_string(),
    }
}

/// Convenience wrapper that fills `now_iso` from the
/// system clock and `engine_version` from
/// `CARGO_PKG_VERSION`. Production code paths use this;
/// tests use `analyze_at` for determinism.
pub fn analyze(
    report: &astroforge_core::image_analysis::report::ImageAnalysisReport,
    project_id: &str,
) -> RecommendationReport {
    let now_iso = now_iso_string();
    let engine_version = env!("CARGO_PKG_VERSION");
    analyze_at(report, project_id, &now_iso, engine_version)
}

/// Build an ISO-8601-ish timestamp string for `created_at`.
/// `datetime` from the standard library gives us
/// `YYYY-MM-DD HH:MM:SS.uuu UTC`; we trim to the second
/// so a test fixture does not have to capture sub-second
/// noise.
pub fn now_iso_string() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    format_unix_seconds(secs)
}

/// Format a unix-second timestamp as `YYYY-MM-DD HH:MM:SS UTC`.
/// Pulled out so `analyze_at` can format arbitrary times.
pub fn format_unix_seconds(secs: i64) -> String {
    // Civil-from-days algorithm by Howard Hinnant (public domain).
    // Avoids pulling in a date crate just for ISO formatting.
    let days = secs.div_euclid(86_400);
    let sod = secs.rem_euclid(86_400);
    let hour = sod / 3600;
    let minute = (sod % 3600) / 60;
    let second = sod % 60;
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    let y = if m <= 2 { y + 1 } else { y };
    format!(
        "{:04}-{:02}-{:02} {:02}:{:02}:{:02} UTC",
        y, m, d, hour, minute, second
    )
}

/// Convert a `RecommendationReport` into the persisted
/// `AiRecommendation` rows the store expects.
///
/// The flat columns (`operation`, `rationale`, `confidence`,
/// `risk_level`) come straight from the report; the
/// §27 payload fields are JSON-serialised into
/// `payload_json`. `created_at` is shared by every row
/// (the engine produces the whole set at once) so the
/// store can correlate them on read-back.
///
/// `recommendation_id` is generated from
/// `(image_version_id, operation, sequence)` so two
/// reports for the same Image Version produce stable ids
/// — re-running the engine on the same analysis overwrites
/// the prior rows via `INSERT OR REPLACE` (P1's store
/// helper does the right thing on collision).
pub fn flatten_for_store(
    report: &RecommendationReport,
) -> Vec<astroforge_core::domain::AiRecommendation> {
    use astroforge_core::domain::AiRecommendation as Row;
    let mut index_by_op: BTreeMap<String, u32> = BTreeMap::new();
    report
        .recommendations
        .iter()
        .map(|r| {
            let seq = index_by_op
                .entry(r.operation.clone())
                .and_modify(|c| *c += 1)
                .or_insert(0);
            let recommendation_id =
                format!("rec_{}_{}_{}", report.image_version_id, r.operation, seq);
            let payload = serde_json::to_string(r).unwrap_or_else(|_| "{}".to_string());
            Row {
                recommendation_id,
                project_id: report.project_id.clone(),
                image_version_id: report.image_version_id.clone(),
                operation: r.operation.clone(),
                rationale: Some(r.reason.clone()),
                confidence: r.confidence,
                risk_level: r.risk_level.clone(),
                payload_json: payload,
                created_at: report.created_at.clone(),
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use astroforge_core::image_analysis::metrics::Confidence;
    use astroforge_core::image_analysis::report::ImageAnalysisReport;

    fn empty_report() -> ImageAnalysisReport {
        ImageAnalysisReport {
            image_version_id: "v1".into(),
            width: 256,
            height: 256,
            channels: 3,
            observations: vec![],
            star_count: 0,
            faint_structure_count: 0,
            has_bright_core: false,
            hot_pixel_count: 0,
            trail_artifact_count: 0,
            engine_version: "test".into(),
            created_at: "2026-01-01 00:00:00 UTC".into(),
        }
    }

    #[test]
    fn empty_analysis_emits_no_recommendations() {
        let report = empty_report();
        let out = analyze_at(&report, "proj1", "2026-01-01 00:00:00 UTC", "0.0.0");
        assert_eq!(out.recommendations.len(), 0);
        assert_eq!(out.sequencing_notes.len(), 0);
        assert_eq!(out.engine_version, "0.0.0");
        assert_eq!(out.created_at, "2026-01-01 00:00:00 UTC");
    }

    #[test]
    fn analyze_is_deterministic_for_same_inputs() {
        let mut report = empty_report();
        report
            .observations
            .push(astroforge_core::image_analysis::report::Observation {
                name: "luminance_noise".into(),
                label: "moderate".into(),
                value: 0.05,
                evidence_region: None,
                confidence: Confidence::Medium,
            });
        let a = analyze_at(&report, "p", "2026-01-01 00:00:00 UTC", "0.0.0");
        let b = analyze_at(&report, "p", "2026-01-01 00:00:00 UTC", "0.0.0");
        assert_eq!(a, b);
    }

    #[test]
    fn flatten_produces_one_row_per_recommendation() {
        let mut report = empty_report();
        report
            .observations
            .push(astroforge_core::image_analysis::report::Observation {
                name: "luminance_noise".into(),
                label: "moderate".into(),
                value: 0.05,
                evidence_region: None,
                confidence: Confidence::Medium,
            });
        report
            .observations
            .push(astroforge_core::image_analysis::report::Observation {
                name: "hot_pixel_count".into(),
                label: "present".into(),
                value: 12.0,
                evidence_region: None,
                confidence: Confidence::High,
            });
        let out = analyze_at(&report, "p1", "2026-01-01 00:00:00 UTC", "0.0.0");
        let rows = flatten_for_store(&out);
        assert_eq!(rows.len(), out.recommendations.len());
        // Each row carries the project_id and image_version_id.
        for r in &rows {
            assert_eq!(r.project_id, "p1");
            assert_eq!(r.image_version_id, "v1");
            assert_eq!(r.created_at, "2026-01-01 00:00:00 UTC");
        }
        // The payload_json is valid JSON that round-trips.
        for r in &rows {
            let parsed: AiRecommendation = serde_json::from_str(&r.payload_json).unwrap();
            assert_eq!(parsed.operation, r.operation);
        }
    }

    #[test]
    fn format_unix_seconds_handles_known_dates() {
        // 2026-01-01 00:00:00 UTC
        assert_eq!(
            format_unix_seconds(1_767_225_600),
            "2026-01-01 00:00:00 UTC"
        );
        // 2000-01-01 00:00:00 UTC
        assert_eq!(format_unix_seconds(946_684_800), "2000-01-01 00:00:00 UTC");
        // Unix epoch
        assert_eq!(format_unix_seconds(0), "1970-01-01 00:00:00 UTC");
    }
}
