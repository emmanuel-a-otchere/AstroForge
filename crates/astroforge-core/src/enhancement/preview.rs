//! CR-06 P4 — Enhancement Preview state machine.
//!
//! A preview is a temporary artifact produced by an
//! `apply` round on a single operation (the apply
//! round is the user pressing "Apply" on the operation
//! card). The artifact is *not* an Image Version — it is
//! a transient state the UI shows in the
//! before/after/split comparison surfaces.
//!
//! The state machine governs the lifecycle:
//!
//! ```text
//! pending → rendering → completed
//!                  │
//!                  └→ failed
//! ```
//!
//! - `pending` — preview row created, the apply round
//!   has not yet started producing pixels.
//! - `rendering` — pixels are being computed (real ONNX
//!   inference in P5; a deterministic placeholder in P4).
//! - `completed` — the artifact is ready; the UI can
//!   show the before/after view.
//! - `failed` — the apply round errored. The UI
//!   surfaces the error in the operation card.
//!
//! The module exposes pure functions for the state
//! transitions; persisting the row is the Tauri
//! command's job (`upsert_enhancement_preview` already
//! exists from P1).
//!
//! P4 deliberately ships the state machine + a
//! deterministic placeholder preview producer so the
//! end-to-end flow is exercisable on a CI runner
//! without a GPU. P5 replaces the placeholder with the
//! real ONNX dispatch.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PreviewStatus {
    Pending,
    Rendering,
    Completed,
    Failed,
}

impl PreviewStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            PreviewStatus::Pending => "pending",
            PreviewStatus::Rendering => "rendering",
            PreviewStatus::Completed => "completed",
            PreviewStatus::Failed => "failed",
        }
    }
}

/// Parse the `status` column back into the typed enum.
/// Unknown strings fall back to `Pending` (defensive —
/// a future schema that adds new states reads as
/// "pending" rather than crashing).
pub fn parse_status(raw: &str) -> PreviewStatus {
    match raw {
        "pending" => PreviewStatus::Pending,
        "rendering" => PreviewStatus::Rendering,
        "completed" => PreviewStatus::Completed,
        "failed" => PreviewStatus::Failed,
        _ => PreviewStatus::Pending,
    }
}

/// A typed view of the `EnhancementPreview` DB row.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EnhancementPreviewRecord {
    pub preview_id: String,
    pub project_id: String,
    pub source_image_version_id: String,
    pub operation_id: String,
    pub artifact_id: Option<String>,
    pub parameters_json: String,
    pub status: PreviewStatus,
    pub created_at: String,
}

/// State-machine transitions. Each function returns the
/// new status; the caller persists it.
pub fn mark_rendering(_preview: &EnhancementPreviewRecord) -> PreviewStatus {
    PreviewStatus::Rendering
}

pub fn mark_completed(artifact_id: String) -> PreviewStatus {
    // The artifact_id is recorded by the apply round; the
    // status transition is the side effect the caller
    // persists alongside it.
    let _ = artifact_id;
    PreviewStatus::Completed
}

pub fn mark_failed(_preview: &EnhancementPreviewRecord) -> PreviewStatus {
    PreviewStatus::Failed
}

/// Produce a deterministic preview payload for an
/// operation. P4 ships a placeholder producer so the
/// end-to-end flow is exercisable without a GPU. The
/// output is a JSON blob describing what the preview
/// would look like:
///
/// - `placeholder_kind` — `"noop" | "passthrough"` (the
///   P4 placeholder just passes the source pixels
///   through with no modification, so the preview is
///   visually identical to the source).
/// - `parameters_hash` — sha256 hex of the parameters
///   JSON; the UI surfaces this so the user can see
///   which parameter set produced the preview.
/// - `note` — `"P4 placeholder: real ONNX inference lands in P5"`.
///
/// P5 replaces this body with the real `astroforge-ai`
/// dispatch (denoise, deconv, star, sr, inpaint).
pub fn produce_placeholder_preview(
    operation_id: &str,
    parameters_json: &str,
) -> PlaceholderPreview {
    PlaceholderPreview {
        placeholder_kind: "passthrough".into(),
        operation_id: operation_id.into(),
        parameters_hash: short_hash(parameters_json),
        note: "P4 placeholder: real ONNX inference lands in P5".into(),
    }
}

/// Lightweight non-cryptographic hash for parameter
/// digest. `sha2` is overkill for a UI display string;
/// FNV-1a produces a stable 64-bit hex that fits on one
/// line.
fn short_hash(input: &str) -> String {
    const FNV_OFFSET: u64 = 0xcbf29ce484222325;
    const FNV_PRIME: u64 = 0x100000001b3;
    let mut h = FNV_OFFSET;
    for byte in input.bytes() {
        h ^= byte as u64;
        h = h.wrapping_mul(FNV_PRIME);
    }
    format!("{:016x}", h)
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlaceholderPreview {
    pub placeholder_kind: String,
    pub operation_id: String,
    pub parameters_hash: String,
    pub note: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_status_round_trips() {
        for s in [
            PreviewStatus::Pending,
            PreviewStatus::Rendering,
            PreviewStatus::Completed,
            PreviewStatus::Failed,
        ] {
            assert_eq!(parse_status(s.as_str()), s);
        }
    }

    #[test]
    fn parse_status_unknown_falls_back_to_pending() {
        assert_eq!(parse_status("unknown"), PreviewStatus::Pending);
    }

    #[test]
    fn state_transitions_are_pure() {
        let p = EnhancementPreviewRecord {
            preview_id: "p1".into(),
            project_id: "pr".into(),
            source_image_version_id: "v1".into(),
            operation_id: "denoise_luminance".into(),
            artifact_id: None,
            parameters_json: "{}".into(),
            status: PreviewStatus::Pending,
            created_at: "2026-01-01 00:00:00 UTC".into(),
        };
        assert_eq!(mark_rendering(&p), PreviewStatus::Rendering);
        assert_eq!(mark_failed(&p), PreviewStatus::Failed);
        assert_eq!(mark_completed("a1".into()), PreviewStatus::Completed);
    }

    #[test]
    fn placeholder_preview_carries_parameters_hash() {
        let p = produce_placeholder_preview("denoise_luminance", r#"{"strength":0.7}"#);
        assert_eq!(p.operation_id, "denoise_luminance");
        assert_eq!(p.placeholder_kind, "passthrough");
        assert_eq!(p.parameters_hash.len(), 16);
        // Same parameters produce the same hash.
        let p2 = produce_placeholder_preview("denoise_luminance", r#"{"strength":0.7}"#);
        assert_eq!(p.parameters_hash, p2.parameters_hash);
        // Different parameters produce different hashes.
        let p3 = produce_placeholder_preview("denoise_luminance", r#"{"strength":0.8}"#);
        assert_ne!(p.parameters_hash, p3.parameters_hash);
    }
}
