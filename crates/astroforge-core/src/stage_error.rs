//! CR-05 P6 slice 1 — §28 Error and Recovery UX structured payloads.
//!
//! Every processing failure should answer four questions (per CR-05 §28):
//!
//! 1. What happened?
//! 2. What was preserved?
//! 3. What can AstroForge do next?
//! 4. What can the user do?
//!
//! This module defines the canonical wire shape that lives inside
//! `StageExecution.error_json` and the helpers that classify a
//! runner failure message into the right suggested actions. The
//! classification is best-effort: when the cause is unknown we
//! emit conservative defaults ("Retry" + "Adjust processing") so
//! the UI never lands in an empty-actions state.
//!
//! Wording for `what_was_preserved` and `what_happened` is plain
//! prose on purpose: the user reads this, not a developer. The
//! structured fields exist for the UI to drive buttons; the prose
//! is for the user to understand.

use serde::{Deserialize, Serialize};

/// One suggested next action, surfaced as a button in the recovery panel.
///
/// `kind` is the semantic category (used by the UI to pick a colour,
/// icon, and handler); `label` is the button text. The two are kept
/// separate so the UI can re-skin buttons without reclassifying
/// every error.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SuggestedAction {
    pub label: String,
    pub kind: SuggestedActionKind,
}

/// Semantic category for `SuggestedAction`. Variant order is the
/// display order in `ErrorRecoveryPanel` — keep stable across
/// versions because the UI sorts on it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SuggestedActionKind {
    /// "Retry Optimized" — same stage, different parameters (smaller
    /// tile, lower precision, etc.).
    RetryOptimized,
    /// "Retry" — same stage, same parameters. The runner just
    /// re-executes with a fresh attempt counter.
    Retry,
    /// "Adjust Processing" — jump back to the stage configuration
    /// panel; user picks different settings.
    AdjustProcessing,
    /// "Skip Stage" — only valid for optional stages; marks the
    /// stage as skipped and resumes from the next.
    SkipStage,
    /// "Cancel" — abort the whole plan.
    Cancel,
    /// "Contact Support" — last-resort action when the error class
    /// is unknown and the engine can't offer anything smarter.
    ContactSupport,
}

/// Canonical structured error payload stored in
/// `StageExecution.error_json` (CR-05 P6.1).
///
/// Round-trips through serde for persistence and for the
/// frontend's `ErrorRecoveryPanel`. The four fields map 1:1 to the
/// four §28 questions — the names are deliberately descriptive
/// because the user will eventually see them in dev tools too.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StageError {
    /// Q1 — what happened (single sentence, user-facing).
    pub what_happened: String,
    /// Q2 — what prior work is still intact.
    pub what_was_preserved: String,
    /// Q3 + Q4 — what the engine offers next + what the user can
    /// do, sorted by `SuggestedActionKind` display order.
    pub suggested_actions: Vec<SuggestedAction>,
}

impl StageError {
    /// Build a structured error from a runner failure.
    ///
    /// `stage_type` is the `PipelineStage.stage_type` of the failed
    /// stage (e.g. `"stack"`, `"stretch"`). `prior_stage_count` is
    /// the number of stages that completed successfully before this
    /// one — used for the "preserved" wording. `msg` is the raw
    /// failure message returned by `dispatch_stage_via_registry`
    /// (currently a free-form string; classification is by keyword).
    pub fn from_failure(stage_type: &str, prior_stage_count: u32, msg: &str) -> Self {
        let classification = classify(msg);
        Self {
            what_happened: render_what_happened(stage_type, msg),
            what_was_preserved: render_what_was_preserved(prior_stage_count, stage_type),
            suggested_actions: classification.actions,
        }
    }

    /// Test-only constructor for cases where the call site has
    /// already classified the error (e.g. a handler that knows it
    /// hit OOM).
    #[cfg(test)]
    pub fn from_parts(
        what_happened: impl Into<String>,
        what_was_preserved: impl Into<String>,
        suggested_actions: Vec<SuggestedAction>,
    ) -> Self {
        Self {
            what_happened: what_happened.into(),
            what_was_preserved: what_was_preserved.into(),
            suggested_actions,
        }
    }
}

/// Internal classification result — what the message looked like
/// and what we should suggest the user do about it.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Classification {
    actions: Vec<SuggestedAction>,
}

/// Heuristic keyword-based classifier. Real implementations
/// should grow a `StageErrorKind` enum that handlers can return
/// explicitly; for now the dispatcher only gives us a free-form
/// message, so we scrape it.
fn classify(msg: &str) -> Classification {
    let lower = msg.to_ascii_lowercase();

    // OOM / memory pressure → retry with smaller tiles.
    if lower.contains("memory")
        || lower.contains("out of memory")
        || lower.contains("oom")
        || lower.contains("alloc")
    {
        return Classification {
            actions: vec![
                SuggestedAction {
                    label: "Retry Optimized".into(),
                    kind: SuggestedActionKind::RetryOptimized,
                },
                SuggestedAction {
                    label: "Adjust Processing".into(),
                    kind: SuggestedActionKind::AdjustProcessing,
                },
                SuggestedAction {
                    label: "Cancel".into(),
                    kind: SuggestedActionKind::Cancel,
                },
            ],
        };
    }

    // IO / file system → check storage.
    if lower.contains("io error")
        || lower.contains("no such file")
        || lower.contains("permission denied")
        || lower.contains("disk")
        || lower.contains("read ")
        || lower.contains("write ")
    {
        return Classification {
            actions: vec![
                SuggestedAction {
                    label: "Retry".into(),
                    kind: SuggestedActionKind::Retry,
                },
                SuggestedAction {
                    label: "Cancel".into(),
                    kind: SuggestedActionKind::Cancel,
                },
            ],
        };
    }

    // Cancelled by user / cancelled by upstream flag → no retry.
    if lower.contains("cancel") {
        return Classification {
            actions: vec![SuggestedAction {
                label: "Adjust Processing".into(),
                kind: SuggestedActionKind::AdjustProcessing,
            }],
        };
    }

    // Unknown — conservative defaults. The UI must always have at
    // least one action so the user isn't stranded.
    Classification {
        actions: vec![
            SuggestedAction {
                label: "Retry".into(),
                kind: SuggestedActionKind::Retry,
            },
            SuggestedAction {
                label: "Adjust Processing".into(),
                kind: SuggestedActionKind::AdjustProcessing,
            },
            SuggestedAction {
                label: "Cancel".into(),
                kind: SuggestedActionKind::Cancel,
            },
        ],
    }
}

fn render_what_happened(stage_type: &str, msg: &str) -> String {
    let stage_label = human_stage_label(stage_type);
    format!("{stage_label} could not complete: {msg}")
}

fn render_what_was_preserved(prior_stage_count: u32, stage_type: &str) -> String {
    let stage_label = human_stage_label(stage_type);
    match prior_stage_count {
        0 => format!(
            "Nothing had run yet — no prior Image Versions were produced before {stage_label} failed."
        ),
        1 => format!(
            "AstroForge successfully preserved 1 Image Version from the stage that ran before {stage_label}."
        ),
        n => format!(
            "AstroForge successfully preserved {n} Image Versions from the stages that ran before {stage_label}."
        ),
    }
}

/// Human-readable stage label. Falls back to a title-cased
/// `stage_type` so unknown stage types still render sensibly.
fn human_stage_label(stage_type: &str) -> String {
    match stage_type {
        "calibrate" => "Calibration".into(),
        "debayer" => "Debayering".into(),
        "register" => "Registration".into(),
        "stack" => "Stacking".into(),
        "background" => "Background extraction".into(),
        "color" => "Color calibration".into(),
        "stretch" => "Stretching".into(),
        "denoise" => "Denoising".into(),
        "detail" => "Detail enhancement".into(),
        "export" => "Exporting".into(),
        _ if stage_type.is_empty() => "Stage".into(),
        _ => {
            stage_type
                .chars()
                .next()
                .map(|c| c.to_ascii_uppercase().to_string())
                .unwrap_or_default()
                + &stage_type[1..]
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classify_oom_offers_retry_optimized_and_cancel() {
        let c = classify("out of memory while building stack");
        assert_eq!(c.actions[0].label, "Retry Optimized");
        assert_eq!(c.actions[0].kind, SuggestedActionKind::RetryOptimized);
        assert!(c
            .actions
            .iter()
            .any(|a| a.kind == SuggestedActionKind::Cancel));
    }

    #[test]
    fn classify_memory_keyword_also_triggers_optimized_retry() {
        // sysinfo returns "memory" not "out of memory" on some
        // platforms; ensure we still catch it.
        let c = classify("insufficient memory: only 512 MiB available");
        assert_eq!(c.actions[0].kind, SuggestedActionKind::RetryOptimized);
    }

    #[test]
    fn classify_io_error_offers_retry_and_cancel() {
        let c = classify("IO error: read failed on /tmp/light.fits");
        assert_eq!(c.actions[0].kind, SuggestedActionKind::Retry);
        assert!(c
            .actions
            .iter()
            .any(|a| a.kind == SuggestedActionKind::Cancel));
    }

    #[test]
    fn classify_no_such_file_offers_retry() {
        let c = classify("no such file or directory: /data/calibrated.fits");
        assert_eq!(c.actions[0].kind, SuggestedActionKind::Retry);
    }

    #[test]
    fn classify_cancel_message_offers_only_adjust_processing() {
        // Cancellation is user-initiated — no point suggesting retry.
        let c = classify("cancelled by user");
        assert_eq!(c.actions.len(), 1);
        assert_eq!(c.actions[0].kind, SuggestedActionKind::AdjustProcessing);
    }

    #[test]
    fn classify_unknown_offers_three_conservative_actions() {
        let c = classify("tensor core mismatch in onnx session");
        assert_eq!(c.actions.len(), 3);
        assert_eq!(c.actions[0].kind, SuggestedActionKind::Retry);
        assert!(c
            .actions
            .iter()
            .any(|a| a.kind == SuggestedActionKind::AdjustProcessing));
        assert!(c
            .actions
            .iter()
            .any(|a| a.kind == SuggestedActionKind::Cancel));
    }

    #[test]
    fn what_happened_includes_stage_label() {
        let err = StageError::from_failure("stack", 2, "out of memory");
        assert!(err.what_happened.starts_with("Stacking"));
        assert!(err.what_happened.contains("out of memory"));
    }

    #[test]
    fn preserved_zero_stages_uses_singular_neutral_wording() {
        let err = StageError::from_failure("calibrate", 0, "anything");
        assert!(err.what_was_preserved.contains("Nothing"));
        assert!(err.what_was_preserved.contains("Calibration"));
    }

    #[test]
    fn preserved_one_stage_uses_singular_count() {
        let err = StageError::from_failure("stack", 1, "anything");
        assert!(err.what_was_preserved.contains("1 Image Version"));
        assert!(!err.what_was_preserved.contains("1 Image Versions"));
    }

    #[test]
    fn preserved_many_stages_uses_plural_count() {
        let err = StageError::from_failure("stretch", 4, "anything");
        assert!(err.what_was_preserved.contains("4 Image Versions"));
    }

    #[test]
    fn unknown_stage_type_is_title_cased() {
        let err = StageError::from_failure("my-custom-stage", 1, "anything");
        // Title-cased to "My-custom-stage" — not pretty, but never
        // blank. UI renderers can override per-stage later.
        assert!(err.what_happened.contains("My-custom-stage"));
    }

    #[test]
    fn empty_stage_type_falls_back_to_generic_stage_label() {
        let err = StageError::from_failure("", 0, "anything");
        assert!(err.what_happened.contains("Stage"));
    }

    #[test]
    fn serde_round_trip_preserves_fields() {
        let err = StageError::from_failure("stack", 2, "out of memory");
        let json = serde_json::to_string(&err).unwrap();
        let parsed: StageError = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, err);
    }

    #[test]
    fn json_shape_matches_canonical_field_names() {
        // Lock the on-disk shape so future schema migrations are
        // explicit. Frontend TS types mirror these names.
        let err = StageError::from_failure("stack", 1, "io error");
        let value: serde_json::Value = serde_json::to_value(&err).unwrap();
        assert!(value.get("what_happened").is_some());
        assert!(value.get("what_was_preserved").is_some());
        assert!(value.get("suggested_actions").is_some());
        assert_eq!(value["suggested_actions"].as_array().unwrap().len(), 2);
    }

    #[test]
    fn from_parts_helper_used_by_tests() {
        let err = StageError::from_parts(
            "boom",
            "kept stuff",
            vec![SuggestedAction {
                label: "Retry".into(),
                kind: SuggestedActionKind::Retry,
            }],
        );
        assert_eq!(err.what_happened, "boom");
        assert_eq!(err.what_was_preserved, "kept stuff");
        assert_eq!(err.suggested_actions.len(), 1);
    }
}
