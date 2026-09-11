//! CR-04 P10 — `classify_target` AI stub.
//!
//! Per D-CR04-3 (CR-04 plan), when the deterministic
//! target detection (P4 `target_detection`) lands below a
//! configurable confidence floor AND the dataset images
//! are small enough to fit in the model context, dispatch
//! to `astroforge_ai::service::classify_target`.
//!
//! In P10 this dispatch is a **stub**: the function
//! returns a `ClassifyTargetResult` that mirrors what a
//! real AI hook would emit, but the AI branch is a no-op
//! pass-through to the deterministic fallback. The
//! `provenance` field on the result distinguishes the
//! deterministic path from the (currently stubbed) AI
//! path so the UI can surface "AI confirming…".
//!
//! ## Why a stub in P10
//!
//! The AI boundary is the seam where AstroForge will
//! eventually call into a real ONNX target-recognition
//! model (CR-04 §9 image-based recognition). The real
//! model lands in a later tranche (CR-04 P5.1 or beyond).
//! P10 ships the seam + the stub so the rest of the
//! pipeline can be exercised end-to-end today without
//! waiting for the model.
//!
//! ## Composes with P4
//!
//! `classify_target` takes the P4 `TargetIntelligence` +
//! the confidence floor. When `intelligence.confidence >=
//! floor`, the stub returns `Deterministic` provenance
//! (no AI work needed). Otherwise it returns
//! `AiStubRequested` so the caller knows the AI path was
//! the source of authority — even though the actual AI
//! model is a no-op today, the seam records the intent.

use serde::{Deserialize, Serialize};

/// CR-04 P10 — the source of authority for the target
/// classification. The UI reads this to surface "AI
/// confirming…" when the path is `AiStubRequested` or a
/// future `AiModel`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TargetClassificationProvenance {
    /// The deterministic P4 classifier was authoritative.
    /// No AI work was performed.
    Deterministic,
    /// The AI stub was requested but the model is a
    /// no-op. The deterministic fallback was used.
    AiStubRequested,
    /// A user override was applied (per §13 ambiguity UX).
    UserOverride,
}

impl TargetClassificationProvenance {
    pub fn as_str(self) -> &'static str {
        match self {
            TargetClassificationProvenance::Deterministic => "deterministic",
            TargetClassificationProvenance::AiStubRequested => "ai_stub_requested",
            TargetClassificationProvenance::UserOverride => "user_override",
        }
    }
}

/// CR-04 P10 — the result of `classify_target`. Mirrors
/// the P4 `TargetIntelligence` shape so the IPC layer can
/// round-trip without a second conversion layer.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ClassifyTargetResult {
    /// The P4 target id (e.g. "M31"), or None if neither
    /// the deterministic nor AI path resolved a target.
    pub target_name: Option<String>,
    /// 0.0..=1.0. Confidence in `target_name`.
    pub confidence: f64,
    pub provenance: TargetClassificationProvenance,
    /// Free-text reasoning surfaced to the UI.
    pub reasoning: String,
}

/// CR-04 P10 — the configurable confidence floor below
/// which the AI stub would be invoked. When the
/// deterministic P4 confidence is above the floor, no AI
/// work is done.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ClassifyTargetConfig {
    /// Default per D-CR04-3: 0.85.
    pub confidence_floor: f64,
    /// When true, the AI stub records `AiStubRequested`
    /// provenance even if the deterministic path would
    /// have sufficed. Useful for testing the AI boundary.
    pub force_ai_path: bool,
}

impl Default for ClassifyTargetConfig {
    fn default() -> Self {
        Self {
            confidence_floor: 0.85,
            force_ai_path: false,
        }
    }
}

/// CR-04 P10 — the input to `classify_target`. Composes
/// over P4's `TargetIntelligence` (re-exported via the
/// consumer) plus the dataset size hint (used to decide
/// whether the dataset fits the AI model context).
#[derive(Debug, Clone)]
pub struct ClassifyTargetInput<'a> {
    /// The session-level `TargetIntelligence` from P4
    /// (the most-confident target across the session's
    /// assets). Re-exported as a typed name to avoid
    /// pulling astroforge-core into the AI module's
    /// public API.
    pub deterministic_target_name: Option<&'a str>,
    pub deterministic_confidence: f64,
    /// Number of assets in the session. The AI stub
    /// refuses when this exceeds `max_assets_for_ai`
    /// because the model context is small.
    pub asset_count: usize,
}

impl<'a> ClassifyTargetInput<'a> {
    pub fn new(
        deterministic_target_name: Option<&'a str>,
        deterministic_confidence: f64,
        asset_count: usize,
    ) -> Self {
        Self {
            deterministic_target_name,
            deterministic_confidence,
            asset_count,
        }
    }
}

/// CR-04 P10 — the canonical AI hook. Pure function. The
/// real ONNX dispatch is out of scope; this stub records
/// the seam so the rest of the pipeline can be exercised
/// end-to-end.
///
/// The function:
/// 1. If `input.deterministic_confidence >= config.confidence_floor`
///    AND `!config.force_ai_path`, returns the
///    deterministic result with `Deterministic` provenance.
/// 2. Otherwise, if `input.asset_count <= 1024` (the
///    current model-context size), records the
///    `AiStubRequested` provenance and returns the
///    deterministic result (the stub doesn't change the
///    answer — it just records the AI boundary).
/// 3. Otherwise (asset_count > 1024), returns the
///    deterministic result with `AiStubRequested`
///    provenance but a reasoning note that the model
///    context is too small. The UI surfaces this as
///    "AI confirming… (context too small — using
///    deterministic fallback)".
pub fn classify_target(
    input: &ClassifyTargetInput<'_>,
    config: &ClassifyTargetConfig,
) -> ClassifyTargetResult {
    let above_floor =
        input.deterministic_confidence >= config.confidence_floor && !config.force_ai_path;

    if above_floor {
        return ClassifyTargetResult {
            target_name: input.deterministic_target_name.map(String::from),
            confidence: input.deterministic_confidence,
            provenance: TargetClassificationProvenance::Deterministic,
            reasoning: format!(
                "Deterministic P4 target detection ({}); confidence above floor {}.",
                input.deterministic_target_name.unwrap_or("(none)"),
                config.confidence_floor
            ),
        };
    }

    // Below floor or AI-forced. Record the intent; the
    // actual AI model is a no-op today.
    let context_too_small = input.asset_count > 1024;
    ClassifyTargetResult {
        target_name: input.deterministic_target_name.map(String::from),
        confidence: input.deterministic_confidence,
        provenance: TargetClassificationProvenance::AiStubRequested,
        reasoning: if context_too_small {
            format!(
                "AI stub requested for {} (confidence {} < floor {}), but asset count {} exceeds the 1024-asset model-context ceiling; falling back to deterministic result.",
                input.deterministic_target_name.unwrap_or("(none)"),
                input.deterministic_confidence,
                config.confidence_floor,
                input.asset_count
            )
        } else {
            format!(
                "AI stub requested for {} (confidence {} < floor {}); AI model is a no-op today, falling back to deterministic result.",
                input.deterministic_target_name.unwrap_or("(none)"),
                input.deterministic_confidence,
                config.confidence_floor
            )
        },
    }
}

// ─── Unit tests ─────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn above_floor_returns_deterministic() {
        let input = ClassifyTargetInput::new(Some("M31"), 0.95, 50);
        let config = ClassifyTargetConfig::default();
        let result = classify_target(&input, &config);
        assert_eq!(
            result.provenance,
            TargetClassificationProvenance::Deterministic
        );
        assert_eq!(result.target_name.as_deref(), Some("M31"));
        assert!((result.confidence - 0.95).abs() < 0.001);
        assert!(result.reasoning.contains("above floor"));
    }

    #[test]
    fn below_floor_records_ai_stub_provenance() {
        let input = ClassifyTargetInput::new(Some("M31"), 0.6, 50);
        let config = ClassifyTargetConfig::default();
        let result = classify_target(&input, &config);
        assert_eq!(
            result.provenance,
            TargetClassificationProvenance::AiStubRequested
        );
        assert!(result.reasoning.contains("stub"));
    }

    #[test]
    fn force_ai_path_overrides_floor() {
        let input = ClassifyTargetInput::new(Some("M31"), 0.95, 50);
        let config = ClassifyTargetConfig {
            confidence_floor: 0.5,
            force_ai_path: true,
        };
        let result = classify_target(&input, &config);
        assert_eq!(
            result.provenance,
            TargetClassificationProvenance::AiStubRequested
        );
    }

    #[test]
    fn large_session_notes_context_too_small() {
        let input = ClassifyTargetInput::new(Some("M31"), 0.6, 5000);
        let config = ClassifyTargetConfig::default();
        let result = classify_target(&input, &config);
        assert_eq!(
            result.provenance,
            TargetClassificationProvenance::AiStubRequested
        );
        assert!(result.reasoning.contains("model-context ceiling"));
    }

    #[test]
    fn unknown_target_returns_none_with_ai_provenance() {
        let input = ClassifyTargetInput::new(None, 0.4, 50);
        let config = ClassifyTargetConfig::default();
        let result = classify_target(&input, &config);
        assert!(result.target_name.is_none());
        assert_eq!(
            result.provenance,
            TargetClassificationProvenance::AiStubRequested
        );
    }

    #[test]
    fn provenance_as_str_round_trips() {
        assert_eq!(
            TargetClassificationProvenance::Deterministic.as_str(),
            "deterministic"
        );
        assert_eq!(
            TargetClassificationProvenance::AiStubRequested.as_str(),
            "ai_stub_requested"
        );
        assert_eq!(
            TargetClassificationProvenance::UserOverride.as_str(),
            "user_override"
        );
    }

    #[test]
    fn default_config_floor_matches_spec() {
        let config = ClassifyTargetConfig::default();
        assert!((config.confidence_floor - 0.85).abs() < 0.001);
        assert!(!config.force_ai_path);
    }
}
