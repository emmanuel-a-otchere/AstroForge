//! CR-05 §7 / §31 — the canonical Stage model.
//!
//! A [`StageSpec`] is the recipe-side definition of a stage (what the
//! Recipe says should happen). A [`PipelineStage`] (in `domain.rs`) is the
//! instantiated form inside a Plan. The plan generator copies a Recipe's
//! stages into a Plan's PipelineStages, applying dataset-aware
//! adaptations (e.g. optional stages may be enabled / disabled based on
//! Session Understanding flags).
//!
//! `StageType` is the canonical identifier shared with the legacy
//! `pipeline::StageType` (in `pipeline.rs` at the crate root) so the P2
//! runner can dispatch without a translation table. Each `StageType`
//! carries a human-readable label, a default `required` flag, and an
//! indication of whether the stage produces a user-visible Image Version.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// CR-05 §31 — canonical stage types. Each entry maps to one engine call
/// (existing module under `crates/astroforge-core/src/`). Keep this list
/// aligned with the legacy `pipeline::StageType` (search for that type's
/// definition to see the parallel list); a translation table is a P2
/// follow-up if drift appears.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StageType {
    Calibrate,
    Debayer,
    QualityFilter,
    Register,
    Stack,
    Background,
    Color,
    Stretch,
    Denoise,
    Detail,
    Export,
}

impl StageType {
    /// Human-readable label per CR-05 §2.1 / §6 Zone A.
    pub fn label(self) -> &'static str {
        match self {
            Self::Calibrate => "Calibrate",
            Self::Debayer => "Debayer",
            Self::QualityFilter => "Quality Filter",
            Self::Register => "Register",
            Self::Stack => "Stack",
            Self::Background => "Background",
            Self::Color => "Color",
            Self::Stretch => "Stretch",
            Self::Denoise => "Denoise",
            Self::Detail => "Detail Enhancement",
            Self::Export => "Export",
        }
    }

    /// Whether the stage is required by default. Required stages ship
    /// enabled; optional stages may be skipped (CR-05 §18).
    pub fn required_by_default(self) -> bool {
        match self {
            Self::Calibrate => true,
            Self::Debayer => true,
            Self::QualityFilter => true,
            Self::Register => true,
            Self::Stack => true,
            Self::Background => false,
            Self::Color => true,
            Self::Stretch => true,
            Self::Denoise => false,
            Self::Detail => false,
            Self::Export => true,
        }
    }

    /// Whether the stage produces a user-visible Image Version (CR-05 §6
    /// Zone A). Quality-filter is interesting: it discards frames rather
    /// than producing a new version, so it returns `false`.
    pub fn produces_image_version_by_default(self) -> bool {
        match self {
            Self::QualityFilter => false,
            Self::Export => false, // Export writes an external artifact, not a new version.
            _ => true,
        }
    }

    /// Whether the stage supports undo (CR-05 §9). Stages that mutate the
    /// source asset (ingest / export) are not undoable.
    pub fn undo_supported(self) -> bool {
        !matches!(self, Self::Export)
    }
}

/// CR-05 §18 — a stage spec as the Recipe defines it. Recipes hold an
/// ordered list of these; the plan generator copies them into a Plan.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StageSpec {
    pub stage_type: StageType,
    pub required: bool,
    pub enabled: bool,
    /// Default parameters. The plan generator may override these from
    /// Session Understanding (P3 will add the recommendation engine).
    #[serde(default)]
    pub parameters: HashMap<String, serde_json::Value>,
}

impl StageSpec {
    pub fn required(stage_type: StageType) -> Self {
        Self {
            stage_type,
            required: stage_type.required_by_default(),
            enabled: true,
            parameters: HashMap::new(),
        }
    }

    pub fn optional(stage_type: StageType) -> Self {
        Self {
            stage_type,
            required: false,
            enabled: false,
            parameters: HashMap::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stage_type_labels_match_spec() {
        // Lock the human-readable labels per CR-05 §6.1. If a stage type
        // is renamed, this test breaks — exactly the surface the UI relies
        // on.
        assert_eq!(StageType::Calibrate.label(), "Calibrate");
        assert_eq!(StageType::Stack.label(), "Stack");
        assert_eq!(StageType::Stretch.label(), "Stretch");
        assert_eq!(StageType::Detail.label(), "Detail Enhancement");
    }

    #[test]
    fn required_defaults() {
        assert!(StageType::Calibrate.required_by_default());
        assert!(StageType::Stretch.required_by_default());
        assert!(!StageType::Background.required_by_default());
        assert!(!StageType::Denoise.required_by_default());
    }

    #[test]
    fn undo_defaults() {
        assert!(StageType::Stretch.undo_supported());
        assert!(!StageType::Export.undo_supported());
    }

    #[test]
    fn stage_spec_constructors() {
        let req = StageSpec::required(StageType::Stack);
        assert!(req.required);
        assert!(req.enabled);

        let opt = StageSpec::optional(StageType::Denoise);
        assert!(!opt.required);
        assert!(!opt.enabled); // optional defaults to disabled until enabled
    }
}
