//! CR-06 P3 — Intelligent ordering (§23).
//!
//! After every rule has emitted its recommendation, the
//! raw list is unsorted. Some orderings are valid
//! (cosmetic operations are commutative) and some are
//! problematic:
//!
//! - **Denoise before detail enhancement.** Detail
//!   enhancement amplifies the local signal — if the
//!   signal still carries noise, the noise gets amplified
//!   too. The engine re-orders any `detail_enhance` row
//!   that lands before `denoise_*`.
//!
//! - **Hot pixel / trail cleanup before denoise.** A
//!   denoise step blurs a 3×3 neighbourhood, so a hot
//!   pixel or trail pixel that survives the cleanup
//!   produces a halo. The engine re-orders
//!   `hot_pixel_clean` and `trail_clean` ahead of
//!   `denoise_*`.
//!
//! - **Background cleanup before star / detail.** A
//!   gradient looks like signal to a star / detail pass.
//!   The engine moves `background_cleanup` to the front.
//!
//! - **Deconv after denoise.** Deconv on noisy data
//!   produces ringing. The engine moves `deconv` after
//!   any `denoise_*` row.
//!
//! - **Star refinement after denoise.** Refining a noisy
//!   star profile sharpens the noise as much as the
//!   star. The engine moves `star_refine` after
//!   `denoise_*`.
//!
//! - **Super-resolution last.** SR is the most expensive
//!   and the most perceptual. The engine parks it at
//!   the end of the list regardless of the original
//!   firing order.
//!
//! The re-ordering is reported back as `SequencingNote`s
//! so the UI can render "Detail enhancement was moved
//! after Denoise — amplifying noise before denoise would
//! bake it in" rather than silently re-ordering.

use serde::{Deserialize, Serialize};

use crate::recommendations::AiRecommendation;

/// A sequencing correction the engine applied.
///
/// `note` is the human-readable rationale the UI
/// surfaces in the recommendation card. `moved` is the
/// operation that was re-positioned; `before_after` is
/// the operation it landed in front of / behind.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SequencingNote {
    pub moved: String,
    pub before_after: String,
    pub note: String,
}

/// Apply the §23 sequencing rules to a flat list of
/// recommendations.
///
/// Returns the re-ordered list (preserving all rows) and
/// the notes the engine emitted during re-ordering. The
/// input list is not mutated; the output is a fresh
/// `Vec`.
pub fn order_recommendations(
    raw: Vec<AiRecommendation>,
) -> (Vec<AiRecommendation>, Vec<SequencingNote>) {
    let mut notes = Vec::new();

    // Strategy: bucket each row by operation class, then
    // stable-sort by bucket with confidence-desc as the
    // within-bucket tiebreaker. Buckets encode the §23
    // ordering:
    //
    // Bucket 0 (front): cleanups — background_cleanup,
    // hot_pixel_clean, trail_clean.
    // Bucket 1: denoise (luminance + chrominance).
    // Bucket 2: post-denoise refinement — star_refine,
    // deconv.
    // Bucket 3 (back): detail_enhance, super_resolution.
    let bucket = |op: &str| -> u8 {
        match op {
            "background_cleanup" | "hot_pixel_clean" | "trail_clean" => 0,
            "denoise_luminance" | "denoise_chrominance" => 1,
            "star_refine" | "deconv" => 2,
            "detail_enhance" | "super_resolution" => 3,
            _ => 1,
        }
    };

    // Capture the original positions before sorting so we
    // can emit notes for any row that moved LATER in the
    // final ordering.
    let original_positions: std::collections::HashMap<String, usize> = raw
        .iter()
        .enumerate()
        .map(|(i, r)| (r.operation.clone(), i))
        .collect();

    let mut out: Vec<AiRecommendation> = raw.into_iter().collect();
    out.sort_by(|a, b| {
        let ba = bucket(&a.operation);
        let bb = bucket(&b.operation);
        ba.cmp(&bb).then_with(|| {
            b.confidence
                .partial_cmp(&a.confidence)
                .unwrap_or(std::cmp::Ordering::Equal)
        })
    });

    // Emit a note whenever the row's final position is
    // later than its original position. The user sees the
    // rationale ("Detail was moved after Denoise — would
    // amplify noise") in the rail.
    for (final_idx, rec) in out.iter().enumerate() {
        let original = original_positions
            .get(&rec.operation)
            .copied()
            .unwrap_or(final_idx);
        if final_idx > original {
            let (note, before_after) = sequencing_rationale(&rec.operation);
            notes.push(SequencingNote {
                moved: rec.operation.clone(),
                before_after: before_after.to_string(),
                note: note.to_string(),
            });
        }
    }

    (out, notes)
}

fn sequencing_rationale(op: &str) -> (&'static str, &'static str) {
    match op {
        "detail_enhance" => (
            "Detail enhancement would amplify noise before denoise had a chance to remove it. \
             The engine moved it after the denoise step so the enhancement reads clean structure.",
            "after denoise_luminance",
        ),
        "star_refine" => (
            "Star refinement would sharpen noise into false star detail. Moved after denoise.",
            "after denoise_luminance",
        ),
        "deconv" => (
            "Deconvolution on noisy data produces ringing. Moved after denoise.",
            "after denoise_luminance",
        ),
        "super_resolution" => (
            "Super-resolution is the most expensive step and the most perceptual — runs last.",
            "at end of stack",
        ),
        "background_cleanup" => (
            "Background gradient removal runs first so subsequent passes treat the sky as flat.",
            "at start of stack",
        ),
        "hot_pixel_clean" => (
            "Hot pixels produce halos under denoise's 3×3 blur. Cleaned before denoise.",
            "before denoise_luminance",
        ),
        "trail_clean" => (
            "Trail pixels leave halos under denoise. Cleaned before denoise.",
            "before denoise_luminance",
        ),
        _ => ("Sequencing adjustment.", "moved"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::recommendations::AiRecommendation;

    fn rec(op: &str, conf: f32) -> AiRecommendation {
        AiRecommendation {
            operation: op.into(),
            reason: "".into(),
            confidence: conf,
            evidence: vec![],
            affected_regions: vec![],
            expected_effect: "".into(),
            risk_level: "low".into(),
            model_candidates: vec![],
            recommended_model: None,
            estimated_runtime: None,
            estimated_memory: None,
            classification: "deterministic".into(),
        }
    }

    #[test]
    fn denoise_lands_before_detail_enhance() {
        // detail_enhance fired first; the engine must
        // re-order it after denoise_luminance and emit a
        // sequencing note explaining why.
        let raw = vec![rec("detail_enhance", 0.95), rec("denoise_luminance", 0.7)];
        let (out, notes) = order_recommendations(raw);
        let positions: Vec<_> = out.iter().map(|r| r.operation.as_str()).collect();
        let denoise = positions
            .iter()
            .position(|op| *op == "denoise_luminance")
            .unwrap();
        let detail = positions
            .iter()
            .position(|op| *op == "detail_enhance")
            .unwrap();
        assert!(
            denoise < detail,
            "denoise must come before detail: {:?}",
            positions
        );
        // detail_enhance had higher confidence but its
        // bucket moved it past denoise — the engine
        // surfaces the reason as a note.
        assert!(
            notes.iter().any(|n| n.moved == "detail_enhance"),
            "expected a sequencing note for detail_enhance, got: {:?}",
            notes
        );
    }

    #[test]
    fn super_resolution_lands_last() {
        let raw = vec![
            rec("super_resolution", 0.9),
            rec("denoise_luminance", 0.9),
            rec("background_cleanup", 0.9),
        ];
        let (out, _) = order_recommendations(raw);
        let last_op = out.last().unwrap().operation.clone();
        assert_eq!(last_op, "super_resolution");
    }

    #[test]
    fn background_cleanup_lands_first() {
        let raw = vec![
            rec("denoise_luminance", 0.9),
            rec("background_cleanup", 0.9),
        ];
        let (out, _) = order_recommendations(raw);
        assert_eq!(out[0].operation, "background_cleanup");
    }

    #[test]
    fn empty_input_produces_empty_output() {
        let (out, notes) = order_recommendations(vec![]);
        assert!(out.is_empty());
        assert!(notes.is_empty());
    }

    #[test]
    fn hot_pixel_clean_lands_before_denoise() {
        let raw = vec![rec("denoise_luminance", 0.9), rec("hot_pixel_clean", 0.9)];
        let (out, _) = order_recommendations(raw);
        let positions: Vec<_> = out.iter().map(|r| r.operation.as_str()).collect();
        let hot = positions
            .iter()
            .position(|op| *op == "hot_pixel_clean")
            .unwrap();
        let denoise = positions
            .iter()
            .position(|op| *op == "denoise_luminance")
            .unwrap();
        assert!(
            hot < denoise,
            "hot pixel clean must come before denoise: {:?}",
            positions
        );
    }

    #[test]
    fn within_bucket_confidence_desc() {
        let raw = vec![
            rec("denoise_luminance", 0.6),
            rec("denoise_chrominance", 0.9),
        ];
        let (out, _) = order_recommendations(raw);
        // Both at bucket 1; chrominance has higher
        // confidence so it sorts first.
        assert_eq!(out[0].operation, "denoise_chrominance");
        assert_eq!(out[1].operation, "denoise_luminance");
    }
}
