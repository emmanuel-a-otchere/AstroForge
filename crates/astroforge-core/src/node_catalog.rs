//! CR-10 P1.6.1.2 / Slice P1.6.1.2:
//! NodeCatalog Rust type + generated manifest.
//!
//! The NodeCatalog is the single source of truth for
//! the user-facing pipeline graph (CR-10 §7.7
//! Node-Based Editor Surface). It enumerates the
//! canonical 12-variant `PipelineStageType` union
//! from `src/lib/pipeline-store.ts` and provides a
//! stable, diff-able JSON manifest at
//! `crates/astroforge-core/src/node_catalog.json`
//! that the TS palette + constrained layout + free-
//! form canvas slices (P1.6.2.x, P1.6.3.x) consume.
//!
//! Why Rust as the source of truth (vs TS)?
//! - The AI boundary classification
//!   (`AiBoundaryLabel::for_stage_type`) already
//!   lives in Rust and is the lock-tested ground
//!   truth. Pinning the catalog to Rust guarantees
//!   the manifest matches the `all_eight_classical_
//!   stage_types_are_not_ai` test at
//!   `ai_boundary.rs:144`.
//! - The destructive + produces_image_version +
//!   undo_supported flags live on
//!   `domain::PipelineStage` and are part of the
//!   execution contract; centralising them here
//!   keeps the catalog in sync with the schema.
//! - The TS side reads the JSON at build time
//!   (via a `nodeCatalog()` helper in P1.6.1.3)
//!   and never authors label/description/destructive
//!   defaults directly.
//!
//! Architecture.
//!
//! - `NodeCatalog` struct: the in-memory catalog.
//!   Holds an ordered `Vec<NodeCatalogEntry>`.
//! - `NodeCatalogEntry` struct: one stage type.
//!   Carries `stage_type` + `label` +
//!   `description` + `ai_label` + `destructive` +
//!   `produces_image_version` + `undo_supported` +
//!   `default_params` + `supported_models`.
//! - `node_catalog()` pure function: returns the
//!   canonical catalog with all 12 entries. Reads
//!   no I/O; deterministic.
//! - `emit_catalog()` helper: writes the catalog to
//!   a JSON file. Used by the `emit-node-catalog`
//!   binary in `src/bin/emit_node_catalog.rs`.
//! - `load_catalog_from_path()`: loads the manifest
//!   at runtime. Used by snapshot tests.
//!
//! The manifest is a checked-in file (not generated
//! at build time). Rationale: a checked-in file is
//! diff-able in git, requires no `build.rs`
//! plumbing, and the `emit-node-catalog` binary
//! gives the canonical regeneration path. The
//! snapshot test (`node_catalog_manifest_matches_
//! generated` in the slice's test file) catches
//! drift between the in-Rust source and the
//! checked-in file.

use crate::ai_boundary::AiBoundaryLabel;
use serde::{Deserialize, Serialize};

/// The user-facing pipeline stage vocabulary.
/// Mirrors the 12-variant `PipelineStageType` union
/// in `src/lib/pipeline-store.ts:8-22`. Keep the
/// two lists in lockstep — the CR-10 §7.7.9
/// mapping table reconciles the 12-user-facing
/// variants to the 8-classical + 2-AI canonical
/// stage types from `AiBoundaryLabel::for_stage_type`.
pub const USER_FACING_STAGE_TYPES: &[&str] = &[
    "ingest",
    "crop_rotate",
    "background_extraction",
    "color_calibration",
    "color_wb",
    "color_scnr",
    "sharpen_deconvolution",
    "denoise",
    "stretch",
    "star_handling",
    "creative_polish",
    "export",
];

/// Canonical 8 classical + 2 AI stage types
/// (the §7.7.9 mapping target vocabulary).
/// Matches `AiBoundaryLabel::for_stage_type` and
/// the `all_eight_classical_stage_types_are_not_ai`
/// test in `ai_boundary.rs:144-163`.
pub const CANONICAL_STAGE_TYPES: &[&str] = &[
    "calibrate",
    "debayer",
    "register",
    "stack",
    "background",
    "color",
    "stretch",
    "export",
    "denoise",
    "detail",
];

/// Per the §7.7.9 mapping: the 12 user-facing
/// types map onto the 10 canonical types. The map
/// is consulted by the catalog builder to assign
/// `AiBoundaryLabel::for_stage_type(canonical)`.
/// Multiple user-facing types can map onto the
/// same canonical type (e.g. `color_calibration`,
/// `color_wb`, `color_scnr` all map onto `color`).
pub fn user_facing_to_canonical(stage_type: &str) -> &'static str {
    match stage_type {
        // Ingest: file load + camera/filter detection;
        // canonical stage is `calibrate` (the
        // upstream analyser). CR-10 §7.7.9 row 1.
        "ingest" => "calibrate",
        // Crop/rotate: framing stage; canonical is
        // `calibrate` (geometry-adjacent). CR-10
        // §7.7.9 row 2.
        "crop_rotate" => "calibrate",
        // Background extraction: gradient/DBE.
        // Canonical `background`. CR-10 §7.7.9 row 3.
        "background_extraction" => "background",
        // Color calibration group: WB + SCNR +
        // calibration all canonical `color`. CR-10
        // §7.7.9 rows 4-6.
        "color_calibration" => "color",
        "color_wb" => "color",
        "color_scnr" => "color",
        // Sharpen/deconvolution: perceptual AI.
        // Canonical `detail`. CR-10 §7.7.9 row 7.
        "sharpen_deconvolution" => "detail",
        // Denoise: perceptual AI. Canonical `denoise`.
        // CR-10 §7.7.9 row 8.
        "denoise" => "denoise",
        // Stretch: histogram transform. Canonical
        // `stretch`. CR-10 §7.7.9 row 9.
        "stretch" => "stretch",
        // Star handling: layer separation. Canonical
        // `detail` (perceptual AI for star
        // detection). CR-10 §7.7.9 row 10.
        "star_handling" => "detail",
        // Creative polish: curves / narrowband mix.
        // Canonical `detail` (perceptual AI for
        // taste). CR-10 §7.7.9 row 11.
        "creative_polish" => "detail",
        // Export: file write. Canonical `export`.
        // CR-10 §7.7.9 row 12.
        "export" => "export",
        // Unknown stage types are an error; the
        // caller must add them to the table before
        // shipping.
        other => panic!("unknown user-facing stage type: {other}"),
    }
}

/// Destructive flag — mirrors
/// `DESTRUCTIVE_STAGES` in
/// `src/lib/pipeline-store.ts:28-35`.
fn is_destructive(stage_type: &str) -> bool {
    matches!(
        stage_type,
        "crop_rotate"
            | "background_extraction"
            | "color_calibration"
            | "sharpen_deconvolution"
            | "denoise"
    )
}

/// Produces-image-version flag — mirrors
/// `domain::PipelineStage::produces_image_version`
/// defaults. Stages that mutate the canvas write a
/// new ImageVersion; read-only analysers + export
/// do not.
fn produces_image_version(stage_type: &str) -> bool {
    matches!(
        stage_type,
        "crop_rotate"
            | "background_extraction"
            | "color_calibration"
            | "color_wb"
            | "color_scnr"
            | "sharpen_deconvolution"
            | "denoise"
            | "stretch"
            | "star_handling"
            | "creative_polish"
    )
}

/// Undo-supported flag — mirrors
/// `domain::PipelineStage::undo_supported`
/// defaults. Stages whose pixels can be restored
/// from params alone (no info loss) are undoable;
/// destructive + generative stages are not.
fn undo_supported(stage_type: &str) -> bool {
    // Crop/rotate is technically reversible from
    // params but UX-feels destructive — flagged
    // undoable here because the geometry can be
    // restored by re-applying the crop. The
    // destructive flag still gates the warning.
    matches!(
        stage_type,
        "crop_rotate" | "color_calibration" | "color_wb" | "color_scnr"
    )
}

/// Default params per stage type — mirrors the
/// `defaultParams` field in
/// `src/lib/pipeline-store.ts:131-192`.
fn default_params(stage_type: &str) -> serde_json::Value {
    match stage_type {
        "ingest" => serde_json::json!({}),
        "crop_rotate" => serde_json::json!({"rotation": 0, "aspectRatio": "free"}),
        "background_extraction" => serde_json::json!({"strength": 0.8, "model": "polynomial"}),
        "color_calibration" => serde_json::json!({"method": "auto", "strength": 0.7}),
        "color_wb" => serde_json::json!({"method": "auto", "strength": 0.7}),
        "color_scnr" => serde_json::json!({"amount": 0.5, "preserveHighlights": true}),
        "sharpen_deconvolution" => {
            serde_json::json!({"algorithm": "richardson_lucy", "iterations": 10})
        }
        "denoise" => serde_json::json!({"strength": 0.5, "method": "swinir"}),
        "stretch" => {
            serde_json::json!({"blackPoint": 0, "midtones": 0.25, "highlights": 1, "mode": "deep"})
        }
        "star_handling" => {
            serde_json::json!({"separationMethod": "exact", "replaceStrength": 1.0, "colorBoost": 0})
        }
        "creative_polish" => serde_json::json!({"saturation": 0, "curves": []}),
        "export" => {
            serde_json::json!({"format": "tiff16", "includeStarless": false, "includeStarsOnly": false})
        }
        other => panic!("unknown stage type: {other}"),
    }
}

/// Label + description — mirrors the `label` and
/// `description` fields in
/// `src/lib/pipeline-store.ts:131-192`.
fn label_and_description(stage_type: &str) -> (&'static str, &'static str) {
    match stage_type {
        "ingest" => (
            "Ingest & Analyse",
            "Load image files, detect camera type, filter set, and basic statistics",
        ),
        "crop_rotate" => (
            "Framing / Crop / Rotate",
            "Interactive crop with live rotation and aspect-ratio presets",
        ),
        "background_extraction" => (
            "Gradient / Background Extraction",
            "Remove sky gradients while preserving nebulosity",
        ),
        "color_calibration" => (
            "Colour Calibration / Balance",
            "Bounded white-balance corrections for dual-band and mono data",
        ),
        "color_wb" => (
            "White Balance",
            "Per-channel white-balance scaling with clipping guard",
        ),
        "color_scnr" => (
            "SCNR Green",
            "Remove green cast from dual-band and RGB narrowband",
        ),
        "sharpen_deconvolution" => (
            "Sharpen / Deconvolution",
            "Richardson-Lucy or van Cittert sharpening with PSF from stars",
        ),
        "denoise" => ("Denoise", "Noise reduction with preview-stable statistics"),
        "stretch" => (
            "Stretch",
            "Data-anchored Deep stretch with multi-preview grid",
        ),
        "star_handling" => (
            "Star Handling",
            "Separate stars, edit layers independently, exact or soft replace",
        ),
        "creative_polish" => (
            "Creative / Final Polish",
            "Curves, colour transmutation spells, narrowband palette mixes",
        ),
        "export" => (
            "Export",
            "Multi-format export: FITS master, TIFF, JPEG, starless, stars-only",
        ),
        other => panic!("unknown stage type: {other}"),
    }
}

/// Per-stage supported models. AI stages list their
/// model IDs; classical stages list an empty vec.
fn supported_models(ai_label: &AiBoundaryLabel) -> Vec<String> {
    if let Some(model_id) = &ai_label.model_id {
        vec![model_id.clone()]
    } else {
        Vec::new()
    }
}

/// One catalog row.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct NodeCatalogEntry {
    /// User-facing stage type, e.g. `color_calibration`.
    pub stage_type: String,
    /// Human-readable label, e.g. `Colour Calibration / Balance`.
    pub label: String,
    /// 1-2 sentence description (shown in palette
    /// + tooltip + parameter panel header).
    pub description: String,
    /// AI classification (delegated to `AiBoundaryLabel`).
    pub ai_uses_ai: bool,
    /// AI model ID when `ai_uses_ai` is true (e.g.
    /// `astroforge_denoise_v1`).
    pub ai_model_id: Option<String>,
    /// Whether the stage triggers the destructive
    /// re-apply warning.
    pub destructive: bool,
    /// Whether the stage writes a new ImageVersion.
    pub produces_image_version: bool,
    /// Whether the stage's pixels can be restored
    /// from params alone (no info loss).
    pub undo_supported: bool,
    /// Default params JSON (consumed by the
    /// Parameter Panel on stage insert).
    pub default_params: serde_json::Value,
    /// Supported AI model IDs (empty for classical).
    pub supported_models: Vec<String>,
}

/// The full catalog.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct NodeCatalog {
    /// Catalog version. Bumped when the schema
    /// changes. The TS-side helper validates
    /// `version` before consuming the manifest.
    pub version: u32,
    /// Catalog entries, in user-facing stage-type
    /// order (matches `USER_FACING_STAGE_TYPES`).
    pub entries: Vec<NodeCatalogEntry>,
}

/// Current catalog version. Bump when fields are
/// added or removed.
pub const CATALOG_VERSION: u32 = 1;

/// Returns the canonical `NodeCatalog`. Pure
/// function; reads no I/O. Deterministic.
pub fn node_catalog() -> NodeCatalog {
    let entries = USER_FACING_STAGE_TYPES
        .iter()
        .map(|stage_type| {
            let canonical = user_facing_to_canonical(stage_type);
            let ai_label = AiBoundaryLabel::for_stage_type(canonical);
            let (label, description) = label_and_description(stage_type);
            NodeCatalogEntry {
                stage_type: (*stage_type).to_string(),
                label: label.to_string(),
                description: description.to_string(),
                ai_uses_ai: ai_label.uses_ai,
                ai_model_id: ai_label.model_id.clone(),
                destructive: is_destructive(stage_type),
                produces_image_version: produces_image_version(stage_type),
                undo_supported: undo_supported(stage_type),
                default_params: default_params(stage_type),
                supported_models: supported_models(&ai_label),
            }
        })
        .collect();
    NodeCatalog {
        version: CATALOG_VERSION,
        entries,
    }
}

/// Emit the catalog to a JSON string. Used by the
/// `emit-node-catalog` binary and the snapshot
/// test. Format: pretty (2-space indent) for
/// diff-ability in git.
pub fn emit_catalog_json() -> String {
    let catalog = node_catalog();
    serde_json::to_string_pretty(&catalog).expect("NodeCatalog serializes to JSON without failure")
}

/// Load a `NodeCatalog` from a JSON string.
pub fn load_catalog_from_str(s: &str) -> Result<NodeCatalog, serde_json::Error> {
    serde_json::from_str(s)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_has_all_twelve_user_facing_entries() {
        let cat = node_catalog();
        assert_eq!(cat.entries.len(), USER_FACING_STAGE_TYPES.len());
        assert_eq!(cat.entries.len(), 12);
        for (i, ty) in USER_FACING_STAGE_TYPES.iter().enumerate() {
            assert_eq!(cat.entries[i].stage_type, *ty);
        }
    }

    #[test]
    fn catalog_version_is_one() {
        let cat = node_catalog();
        assert_eq!(cat.version, 1);
    }

    #[test]
    fn every_entry_has_non_empty_label_and_description() {
        let cat = node_catalog();
        for e in &cat.entries {
            assert!(
                !e.label.is_empty(),
                "entry {} has empty label",
                e.stage_type
            );
            assert!(
                !e.description.is_empty(),
                "entry {} has empty description",
                e.stage_type
            );
        }
    }

    #[test]
    fn denoise_and_detail_user_facing_types_are_marked_ai() {
        let cat = node_catalog();
        let denoise = cat
            .entries
            .iter()
            .find(|e| e.stage_type == "denoise")
            .expect("denoise entry present");
        assert!(denoise.ai_uses_ai);
        assert_eq!(
            denoise.ai_model_id.as_deref(),
            Some("astroforge_denoise_v1")
        );
        let sharpen = cat
            .entries
            .iter()
            .find(|e| e.stage_type == "sharpen_deconvolution")
            .expect("sharpen entry present");
        assert!(sharpen.ai_uses_ai);
        assert_eq!(
            sharpen.ai_model_id.as_deref(),
            Some("astroforge_detail_v1.2")
        );
        let star = cat
            .entries
            .iter()
            .find(|e| e.stage_type == "star_handling")
            .expect("star_handling entry present");
        assert!(star.ai_uses_ai);
        assert_eq!(star.ai_model_id.as_deref(), Some("astroforge_detail_v1.2"));
        let polish = cat
            .entries
            .iter()
            .find(|e| e.stage_type == "creative_polish")
            .expect("creative_polish entry present");
        assert!(polish.ai_uses_ai);
        assert_eq!(
            polish.ai_model_id.as_deref(),
            Some("astroforge_detail_v1.2")
        );
    }

    #[test]
    fn classical_user_facing_types_are_not_ai() {
        // AI types (perceptual): denoise, sharpen_deconvolution
        // (detail), star_handling (detail), creative_polish (detail).
        // Everything else is classical deterministic processing.
        let cat = node_catalog();
        let classical_types = [
            "ingest",
            "crop_rotate",
            "background_extraction",
            "color_calibration",
            "color_wb",
            "color_scnr",
            "stretch",
            "export",
        ];
        for ty in classical_types {
            let entry = cat
                .entries
                .iter()
                .find(|e| e.stage_type == ty)
                .unwrap_or_else(|| panic!("{ty} entry present"));
            assert!(!entry.ai_uses_ai, "{ty} must not be AI");
            assert_eq!(entry.ai_model_id, None, "{ty} must have no model_id");
        }
    }

    #[test]
    fn destructive_set_matches_typescript_destructive_stages() {
        // Mirrors `DESTRUCTIVE_STAGES` in
        // `src/lib/pipeline-store.ts:28-35`.
        let cat = node_catalog();
        let destructive: Vec<&str> = cat
            .entries
            .iter()
            .filter(|e| e.destructive)
            .map(|e| e.stage_type.as_str())
            .collect();
        assert_eq!(
            destructive,
            vec![
                "crop_rotate",
                "background_extraction",
                "color_calibration",
                "sharpen_deconvolution",
                "denoise",
            ]
        );
    }

    #[test]
    fn ingest_and_export_do_not_produce_image_version() {
        let cat = node_catalog();
        let ingest = cat
            .entries
            .iter()
            .find(|e| e.stage_type == "ingest")
            .unwrap();
        assert!(!ingest.produces_image_version);
        let export = cat
            .entries
            .iter()
            .find(|e| e.stage_type == "export")
            .unwrap();
        assert!(!export.produces_image_version);
    }

    #[test]
    fn emitting_and_loading_round_trip() {
        let json = emit_catalog_json();
        let parsed = load_catalog_from_str(&json).expect("manifest parses");
        let re_emitted = serde_json::to_string_pretty(&parsed).unwrap();
        assert_eq!(json, re_emitted);
    }

    #[test]
    fn default_params_are_objects_not_null() {
        let cat = node_catalog();
        for e in &cat.entries {
            assert!(
                e.default_params.is_object(),
                "{} default_params must be a JSON object, got: {}",
                e.stage_type,
                e.default_params
            );
        }
    }

    #[test]
    fn user_facing_to_canonical_is_total() {
        for ty in USER_FACING_STAGE_TYPES {
            // Must not panic.
            let canonical = user_facing_to_canonical(ty);
            assert!(
                CANONICAL_STAGE_TYPES.contains(&canonical),
                "user-facing {ty} mapped to non-canonical {canonical}"
            );
        }
    }
}
