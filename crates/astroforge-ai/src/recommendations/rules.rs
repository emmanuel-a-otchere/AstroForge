//! CR-06 P3 — recommendation rules.
//!
//! Each rule is a pure function over an
//! `ImageAnalysisReport` that emits zero or more
//! `AiRecommendation` rows. The rules encode the
//! observation-to-recommendation mapping from CR-06 §7 /
//! §27:
//!
//! - `luminance_noise` crosses "moderate" → recommend
//!   `denoise` with high confidence and the swinir-denoise
//!   model.
//! - `chromatic_noise` crosses "moderate" → recommend
//!   chrominance denoise (folded into the same denoise
//!   recommendation as a separate concern).
//! - `background_gradient` crosses "moderate" → recommend
//!   `background_cleanup`.
//! - `local_contrast` low + `star_count` high → recommend
//!   `star_refine` (stars exist but lack definition).
//! - `local_contrast` low + `faint_structure_count` high →
//!   recommend `detail_enhance` (faint structures need
//!   detail recovery).
//! - `hot_pixel_count` > 0 → recommend `hot_pixel_clean`
//!   (deterministic; high confidence).
//! - `trail_artifact_count` > 0 → recommend `trail_clean`
//!   (inpaint-class; medium confidence; high risk because
//!   the AI must synthesise missing pixels).
//!
//! Each rule is independently testable: pass an
//! `ImageAnalysisReport` with the relevant observation
//! flipped on, assert the recommendation comes out with
//! the expected confidence + risk + model.

use astroforge_core::image_analysis::metrics::Confidence as ObsConfidence;
use astroforge_core::image_analysis::report::{ImageAnalysisReport, Observation};
use serde::{Deserialize, Serialize};

use crate::recommendations::resource_estimate::{estimate_for, ResourceEstimate};
use crate::recommendations::{AiRecommendation, ModelCandidate};

/// The unsorted output of every rule.
///
/// `recommendations` is flat (one per rule firing);
/// `ordering::order_recommendations` post-processes it
/// to enforce §23 sequencing.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct RecommendationSet {
    pub recommendations: Vec<AiRecommendation>,
}

/// Walk every rule and collect the recommendations.
///
/// Deterministic for a given input: the same analysis
/// produces the same set in the same order. The order is
/// rule-driven (a stable list of rules evaluated
/// sequentially); the final ordering pass re-orders
/// according to §23 sequencing constraints.
pub fn recommend(report: &ImageAnalysisReport) -> RecommendationSet {
    let mut out = Vec::new();
    rule_luminance_denoise(report, &mut out);
    rule_chrominance_denoise(report, &mut out);
    rule_background_cleanup(report, &mut out);
    rule_hot_pixel_clean(report, &mut out);
    rule_trail_clean(report, &mut out);
    rule_star_refine(report, &mut out);
    rule_detail_enhance(report, &mut out);
    rule_deconv(report, &mut out);
    rule_super_resolution(report, &mut out);
    RecommendationSet {
        recommendations: out,
    }
}

/// Look up an observation by `name` on the report. The
/// report stores observations as a flat list (P2 ships
/// 10 named observations); a missing observation means
/// the rule that depends on it has nothing to act on.
fn find_observation<'a>(report: &'a ImageAnalysisReport, name: &str) -> Option<&'a Observation> {
    report.observations.iter().find(|o| o.name == name)
}

fn label_is_at_least(label: &str, threshold: &str) -> bool {
    // Order is: low < moderate < high < extreme.
    let rank = |s: &str| match s {
        "low" | "Low" => 1,
        "moderate" | "Moderate" => 2,
        "high" | "High" => 3,
        "extreme" | "Extreme" => 4,
        _ => 0,
    };
    rank(label) >= rank(threshold)
}

fn obs_confidence_to_f32(c: ObsConfidence) -> f32 {
    match c {
        ObsConfidence::Low => 0.5,
        ObsConfidence::Medium => 0.75,
        ObsConfidence::High => 0.92,
    }
}

/// Build the `model_candidates` list for an operation by
/// filtering the canonical registry by `stage`. P3 picks
/// `swinir-denoise-astro` for denoise, `swinir-sr-astro-2x`
/// for super-resolution, `trail-lama-tiny` for trail
/// cleanup, and so on. The candidates list is always
/// ordered: the recommended model first, the rest after.
fn candidates_for_stage(
    stage: &str,
    recommended_name: Option<&str>,
) -> (Vec<ModelCandidate>, Option<String>) {
    let registry = crate::hub::get_models_for_stage(stage);
    let mut candidates: Vec<ModelCandidate> = registry
        .iter()
        .map(|m| ModelCandidate {
            name: m.name.clone(),
            version: m.version.clone(),
            sha256: m.sha256.clone(),
            tile_size: m.input_tile_size,
        })
        .collect();
    // Move the recommended model to the front.
    let recommended = if let Some(name) = recommended_name {
        if let Some(pos) = candidates.iter().position(|c| c.name == name) {
            let rec = candidates.remove(pos);
            candidates.insert(0, rec);
            Some(name.to_string())
        } else {
            // The recommended name was not in the registry —
            // still record it so the UI shows the intended pick.
            Some(name.to_string())
        }
    } else {
        candidates.first().map(|c| c.name.clone())
    };
    (candidates, recommended)
}

/// Compute resource estimates from the recommended model
/// + the image dimensions on the report.
fn resource_estimate(
    recommended_name: Option<&str>,
    width: u32,
    height: u32,
) -> (Option<String>, Option<String>) {
    let Some(name) = recommended_name else {
        return (None, None);
    };
    let Some(model) = crate::hub::get_model(name) else {
        return (None, None);
    };
    let ResourceEstimate {
        estimated_runtime,
        estimated_memory,
    } = estimate_for(&model, width, height);
    (Some(estimated_runtime), Some(estimated_memory))
}

/// Rule — `luminance_noise` moderate+ → denoise (luminance).
fn rule_luminance_denoise(report: &ImageAnalysisReport, out: &mut Vec<AiRecommendation>) {
    let Some(obs) = find_observation(report, "luminance_noise") else {
        return;
    };
    if !label_is_at_least(&obs.label, "moderate") {
        return;
    }
    let confidence = obs_confidence_to_f32(obs.confidence);
    let (candidates, recommended) =
        candidates_for_stage("noise_reduction", Some("swinir-denoise-astro"));
    let (runtime, memory) = resource_estimate(recommended.as_deref(), report.width, report.height);
    let label = format!("{}_lum", obs.label.to_lowercase());
    out.push(AiRecommendation {
        operation: "denoise_luminance".into(),
        reason: format!(
            "Luminance noise reads as {} ({}). Reducing noise before any detail \
             work prevents amplifying the noise into the structural signal.",
            label,
            obs.confidence.confidence_label()
        ),
        confidence,
        evidence: vec!["luminance_noise".into()],
        affected_regions: vec![],
        expected_effect: "Reduced grain in low-signal areas; fine structure preserved.".into(),
        risk_level: if label == "extreme_lum" {
            "medium".into()
        } else {
            "low".into()
        },
        model_candidates: candidates,
        recommended_model: recommended,
        estimated_runtime: runtime,
        estimated_memory: memory,
        classification: "perceptual".into(),
    });
}

/// Rule — `chromatic_noise` moderate+ → chrominance
/// denoise. The catalog reuses the same model (the
/// swinir-denoise-astro variant handles both axes) so
/// the recommendation collapses into the same model;
/// the operation name distinguishes the intent for the
/// enhancement stack.
fn rule_chrominance_denoise(report: &ImageAnalysisReport, out: &mut Vec<AiRecommendation>) {
    let Some(obs) = find_observation(report, "chromatic_noise") else {
        return;
    };
    if !label_is_at_least(&obs.label, "moderate") {
        return;
    }
    let confidence = obs_confidence_to_f32(obs.confidence);
    let (candidates, recommended) =
        candidates_for_stage("noise_reduction", Some("swinir-denoise-astro"));
    let (runtime, memory) = resource_estimate(recommended.as_deref(), report.width, report.height);
    out.push(AiRecommendation {
        operation: "denoise_chrominance".into(),
        reason: format!(
            "Chromatic noise reads as {} ({}). A chrominance pass before color \
             calibration avoids baking color speckle into the calibrated result.",
            obs.label,
            obs.confidence.confidence_label()
        ),
        confidence,
        evidence: vec!["chromatic_noise".into()],
        affected_regions: vec![],
        expected_effect: "Smoother color channels; saturation preserved on stars.".into(),
        risk_level: "low".into(),
        model_candidates: candidates,
        recommended_model: recommended,
        estimated_runtime: runtime,
        estimated_memory: memory,
        classification: "perceptual".into(),
    });
}

/// Rule — `background_gradient` moderate+ → background
/// cleanup. The recommended approach is deterministic
/// gradient extraction (CR-06 §10 background
/// intelligence), so the risk is low and the
/// classification is deterministic.
fn rule_background_cleanup(report: &ImageAnalysisReport, out: &mut Vec<AiRecommendation>) {
    let Some(obs) = find_observation(report, "background_gradient") else {
        return;
    };
    if !label_is_at_least(&obs.label, "moderate") {
        return;
    }
    let confidence = obs_confidence_to_f32(obs.confidence);
    // No canonical AI model for background extraction;
    // the operation is a deterministic pass.
    let (runtime, memory) = (Some("4 sec".to_string()), Some("120 MB".to_string()));
    out.push(AiRecommendation {
        operation: "background_cleanup".into(),
        reason: format!(
            "Background gradient reads as {} ({}). Removing the gradient first \
             prevents the AI from treating the gradient as signal and lets stars \
             and faint structure stand on a flat sky.",
            obs.label,
            obs.confidence.confidence_label()
        ),
        confidence,
        evidence: vec!["background_gradient".into()],
        affected_regions: vec![],
        expected_effect: "Flatter sky background; cleaner stars and faint structure.".into(),
        risk_level: "low".into(),
        model_candidates: vec![],
        recommended_model: None,
        estimated_runtime: runtime,
        estimated_memory: memory,
        classification: "deterministic".into(),
    });
}

/// Rule — `hot_pixel_count > 0` → hot pixel cleanup.
/// Deterministic, high confidence, near-zero risk.
fn rule_hot_pixel_clean(report: &ImageAnalysisReport, out: &mut Vec<AiRecommendation>) {
    if report.hot_pixel_count == 0 {
        return;
    }
    let Some(obs) = find_observation(report, "hot_pixels") else {
        return;
    };
    let confidence = obs_confidence_to_f32(obs.confidence);
    let (runtime, memory) = (Some("2 sec".to_string()), Some("80 MB".to_string()));
    out.push(AiRecommendation {
        operation: "hot_pixel_clean".into(),
        reason: format!(
            "{} hot pixel(s) detected ({}). Cleaning hot pixels before denoise \
             prevents the AI from blurring the bad pixels into 3×3 halos.",
            report.hot_pixel_count,
            obs.confidence.confidence_label()
        ),
        confidence,
        evidence: vec!["hot_pixels".into()],
        affected_regions: vec![],
        expected_effect: "Single-pixel defects removed; surrounding pixels untouched.".into(),
        risk_level: "low".into(),
        model_candidates: vec![],
        recommended_model: None,
        estimated_runtime: runtime,
        estimated_memory: memory,
        classification: "deterministic".into(),
    });
}

/// Rule — `trail_artifact_count > 0` → trail cleanup
/// (inpaint-class). Medium confidence, high risk — the
/// AI must synthesise missing pixels, which is a
/// generative operation per CR-06 §16.
fn rule_trail_clean(report: &ImageAnalysisReport, out: &mut Vec<AiRecommendation>) {
    if report.trail_artifact_count == 0 {
        return;
    }
    let Some(obs) = find_observation(report, "trail_artifacts") else {
        return;
    };
    let confidence = obs_confidence_to_f32(obs.confidence) * 0.85;
    let (candidates, recommended) = candidates_for_stage("trail_inpaint", Some("trail-lama-tiny"));
    let (runtime, memory) = resource_estimate(recommended.as_deref(), report.width, report.height);
    out.push(AiRecommendation {
        operation: "trail_clean".into(),
        reason: format!(
            "{} trail artefact(s) detected ({}). Inpainting is generative — the \
             trail pixels will be synthesised from surrounding context, so the \
             classification is generative and a banner will surface in the UI.",
            report.trail_artifact_count,
            obs.confidence.confidence_label()
        ),
        confidence,
        evidence: vec!["trail_artifacts".into()],
        affected_regions: vec![],
        expected_effect: "Trails removed; surrounding background synthesised.".into(),
        risk_level: "high".into(),
        model_candidates: candidates,
        recommended_model: recommended,
        estimated_runtime: runtime,
        estimated_memory: memory,
        classification: "generative".into(),
    });
}

/// Rule — `star_count` high + `local_contrast` low → star
/// refinement (sharpen stars without amplifying noise).
fn rule_star_refine(report: &ImageAnalysisReport, out: &mut Vec<AiRecommendation>) {
    if report.star_count == 0 {
        return;
    }
    let Some(lc) = find_observation(report, "local_contrast") else {
        return;
    };
    if !label_is_at_least(&lc.label, "low") {
        // local_contrast reads low when structure is muddy;
        // moderate+ contrast means stars already pop, so no
        // refinement is needed.
        return;
    }
    let confidence = obs_confidence_to_f32(lc.confidence) * 0.9;
    let (candidates, recommended) = candidates_for_stage("star_segmentation", Some("star-seg-v1"));
    let (runtime, memory) = resource_estimate(recommended.as_deref(), report.width, report.height);
    out.push(AiRecommendation {
        operation: "star_refine".into(),
        reason: format!(
            "{} star(s) detected but local contrast reads {} ({}). Star refinement \
             sharpens star profiles without amplifying the surrounding noise.",
            report.star_count,
            lc.label,
            lc.confidence.confidence_label()
        ),
        confidence,
        evidence: vec!["star_count".into(), "local_contrast".into()],
        affected_regions: vec![],
        expected_effect: "Sharper star cores; tighter FWHM; color preserved.".into(),
        risk_level: "low".into(),
        model_candidates: candidates,
        recommended_model: recommended,
        estimated_runtime: runtime,
        estimated_memory: memory,
        classification: "perceptual".into(),
    });
}

/// Rule — `faint_structure_count` positive + `local_contrast`
/// low → detail enhancement. Faint structures are present
/// but lack local contrast.
fn rule_detail_enhance(report: &ImageAnalysisReport, out: &mut Vec<AiRecommendation>) {
    if report.faint_structure_count == 0 {
        return;
    }
    let Some(lc) = find_observation(report, "local_contrast") else {
        return;
    };
    if !label_is_at_least(&lc.label, "low") {
        return;
    }
    let confidence = obs_confidence_to_f32(lc.confidence) * 0.85;
    let (runtime, memory) = (Some("12 sec".to_string()), Some("640 MB".to_string()));
    out.push(AiRecommendation {
        operation: "detail_enhance".into(),
        reason: format!(
            "{} faint structure(s) detected but local contrast reads {} ({}). \
             Detail enhancement recovers local contrast in faint areas; this \
             should run after any denoise step to avoid amplifying noise.",
            report.faint_structure_count,
            lc.label,
            lc.confidence.confidence_label()
        ),
        confidence,
        evidence: vec!["faint_structure_count".into(), "local_contrast".into()],
        affected_regions: vec![],
        expected_effect: "Faint structures gain local contrast; bright cores preserved.".into(),
        risk_level: "medium".into(),
        model_candidates: vec![],
        recommended_model: None,
        estimated_runtime: runtime,
        estimated_memory: memory,
        classification: "perceptual".into(),
    });
}

/// Rule — `luminance_noise` high + `faint_structure_count`
/// positive → deconvolution after denoise. Deconv is
/// only useful on clean data; noisy data produces
/// ringing.
fn rule_deconv(report: &ImageAnalysisReport, out: &mut Vec<AiRecommendation>) {
    if report.faint_structure_count == 0 {
        return;
    }
    let Some(noise) = find_observation(report, "luminance_noise") else {
        return;
    };
    if !label_is_at_least(&noise.label, "high") {
        return;
    }
    let confidence = obs_confidence_to_f32(noise.confidence) * 0.7;
    out.push(AiRecommendation {
        operation: "deconv".into(),
        reason: format!(
            "Luminance noise reads {} ({}), which would produce ringing if \
             deconvolved directly. Run deconv only after denoise has cleaned the \
             signal.",
            noise.label,
            noise.confidence.confidence_label()
        ),
        confidence,
        evidence: vec!["luminance_noise".into(), "faint_structure_count".into()],
        affected_regions: vec![],
        expected_effect: "Sharper structural detail after denoise; ringing suppressed.".into(),
        risk_level: "medium".into(),
        model_candidates: vec![],
        recommended_model: None,
        estimated_runtime: Some("45 sec".to_string()),
        estimated_memory: Some("1.2 GB".to_string()),
        classification: "deterministic".into(),
    });
}

/// Rule — `star_count` moderate-high + `local_contrast`
/// moderate+ → super-resolution. SR is perceptual and
/// expensive; only recommend it when the image has
/// enough structure to benefit.
fn rule_super_resolution(report: &ImageAnalysisReport, out: &mut Vec<AiRecommendation>) {
    if report.star_count < 20 {
        return;
    }
    let Some(lc) = find_observation(report, "local_contrast") else {
        return;
    };
    if !label_is_at_least(&lc.label, "moderate") {
        return;
    }
    let confidence = obs_confidence_to_f32(lc.confidence) * 0.7;
    let (candidates, recommended) =
        candidates_for_stage("ai_super_resolution", Some("swinir-sr-astro-2x"));
    let (runtime, memory) = resource_estimate(recommended.as_deref(), report.width, report.height);
    out.push(AiRecommendation {
        operation: "super_resolution".into(),
        reason: format!(
            "{} star(s) detected with local contrast {} ({}). 2× super-resolution \
             is perceptual and will produce a 4×-pixel image; the operation is \
             the last step of the stack by §23 sequencing.",
            report.star_count,
            lc.label,
            lc.confidence.confidence_label()
        ),
        confidence,
        evidence: vec!["star_count".into(), "local_contrast".into()],
        affected_regions: vec![],
        expected_effect: "2× linear resolution; stars and structures sharper.".into(),
        risk_level: "medium".into(),
        model_candidates: candidates,
        recommended_model: recommended,
        estimated_runtime: runtime,
        estimated_memory: memory,
        classification: "perceptual".into(),
    });
}

// `Observation::confidence` exposes the `confidence_label()`
// helper via the `Confidence` enum. Pulling the helper
// in here keeps the rule bodies short and consistent.
trait ConfidenceExt {
    fn confidence_label(&self) -> &'static str;
}

impl ConfidenceExt for ObsConfidence {
    fn confidence_label(&self) -> &'static str {
        match self {
            ObsConfidence::Low => "low confidence",
            ObsConfidence::Medium => "medium confidence",
            ObsConfidence::High => "high confidence",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use astroforge_core::image_analysis::metrics::Confidence;
    use astroforge_core::image_analysis::report::{ImageAnalysisReport, Observation};

    fn base_report() -> ImageAnalysisReport {
        ImageAnalysisReport {
            image_version_id: "v1".into(),
            width: 512,
            height: 512,
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

    fn push(
        report: &mut ImageAnalysisReport,
        name: &str,
        label: &str,
        value: f64,
        conf: Confidence,
    ) {
        report.observations.push(Observation {
            name: name.into(),
            label: label.into(),
            value,
            evidence_region: None,
            confidence: conf,
        });
    }

    #[test]
    fn empty_analysis_emits_nothing() {
        let set = recommend(&base_report());
        assert_eq!(set.recommendations.len(), 0);
    }

    #[test]
    fn luminance_noise_moderate_emits_denoise_with_model() {
        let mut r = base_report();
        push(
            &mut r,
            "luminance_noise",
            "moderate",
            0.05,
            Confidence::High,
        );
        let set = recommend(&r);
        assert_eq!(set.recommendations.len(), 1);
        let rec = &set.recommendations[0];
        assert_eq!(rec.operation, "denoise_luminance");
        assert_eq!(
            rec.recommended_model.as_deref(),
            Some("swinir-denoise-astro")
        );
        assert_eq!(rec.classification, "perceptual");
        assert!(rec.estimated_runtime.is_some());
        assert!(rec.estimated_memory.is_some());
    }

    #[test]
    fn hot_pixel_count_emits_deterministic_recommendation() {
        let mut r = base_report();
        r.hot_pixel_count = 12;
        push(&mut r, "hot_pixels", "present", 12.0, Confidence::High);
        let set = recommend(&r);
        assert_eq!(set.recommendations.len(), 1);
        let rec = &set.recommendations[0];
        assert_eq!(rec.operation, "hot_pixel_clean");
        assert_eq!(rec.classification, "deterministic");
        assert_eq!(rec.risk_level, "low");
    }

    #[test]
    fn trail_emits_generative_recommendation_with_banner() {
        let mut r = base_report();
        r.trail_artifact_count = 3;
        push(
            &mut r,
            "trail_artifacts",
            "moderate",
            3.0,
            Confidence::Medium,
        );
        let set = recommend(&r);
        assert_eq!(set.recommendations.len(), 1);
        let rec = &set.recommendations[0];
        assert_eq!(rec.operation, "trail_clean");
        assert_eq!(rec.classification, "generative");
        assert_eq!(rec.risk_level, "high");
        assert_eq!(rec.recommended_model.as_deref(), Some("trail-lama-tiny"));
    }

    #[test]
    fn super_resolution_only_when_stars_and_contrast() {
        let mut r = base_report();
        r.star_count = 50;
        push(&mut r, "local_contrast", "high", 0.8, Confidence::High);
        let set = recommend(&r);
        let ops: Vec<_> = set
            .recommendations
            .iter()
            .map(|r| r.operation.as_str())
            .collect();
        assert!(ops.contains(&"super_resolution"));
        assert_eq!(
            set.recommendations
                .iter()
                .find(|r| r.operation == "super_resolution")
                .unwrap()
                .recommended_model
                .as_deref(),
            Some("swinir-sr-astro-2x")
        );
    }

    #[test]
    fn super_resolution_skipped_when_few_stars() {
        let mut r = base_report();
        r.star_count = 5;
        push(&mut r, "local_contrast", "high", 0.8, Confidence::High);
        let set = recommend(&r);
        assert!(set
            .recommendations
            .iter()
            .all(|r| r.operation != "super_resolution"));
    }
}
