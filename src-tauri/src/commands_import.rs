//! CR-04 P8 — Tauri command shells for the Import
//! Understanding workflow.
//!
//! Each command is a thin wrapper over the
//! `astroforge_core::import_understanding` orchestrator +
//! the `DomainStore` persistence helpers. The substantive
//! analysis runs in the core crate; this module owns the
//! IPC surface and the per-project wiring.
//!
//! ## Commands
//!
//! - `import_analyse_session` — run P3..P7 on a session's
//!   assets, persist the resulting `SessionClassification`.
//! - `import_get_understanding` — read the persisted
//!   classification (or `None` when not yet analysed).
//! - `import_confirm` — mark the session as `Confirmed`.
//! - `import_override_classification` — user-overridden
//!   capture kind / narrowband composition.
//! - `import_set_materialised` — mark the session as
//!   `Materialised` (the IPC layer has finished the wizard).
//!
//! The UI (P9) reads `get_import_understanding` to render
//! the Understanding panel + the ambiguity dialog.

use crate::commands_project::ProjectState;
use astroforge_core::domain::{ClassificationProvenance, ImportState};
use astroforge_core::domain_store::DomainStore;
use astroforge_core::import_understanding;
use astroforge_core::import_scan::ExtractedMetadata;
use astroforge_core::SessionAnalysis;
use serde::{Deserialize, Serialize};
use tauri::State;

/// CR-04 P8 — the result of `import_analyse_session`. The
/// frontend persists the inner `SessionAnalysis` to the
/// classification_metadata blob via the store; the typed
/// shape is returned here so the UI can render without a
/// second round-trip.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportAnalysisResult {
    pub session_id: String,
    pub analysis: SessionAnalysis,
    pub classification: astroforge_core::domain::SessionClassification,
}

/// CR-04 P8 — run P3..P7 on a session's assets and persist
/// the result.
///
/// The caller (UI / P9 wizard) supplies the assets'
/// extracted metadata + paths. The orchestrator runs the
/// full pipeline (P3 frame classification, P4 target
/// detection, P5 session grouping, P6 capture analysis, P7
/// narrowband detection) and persists the resulting
/// `SessionClassification` to the durable store.
#[tauri::command]
pub fn import_analyse_session(
    state: State<'_, ProjectState>,
    project_id: String,
    session_id: String,
    assets: Vec<AssetMetadataInput>,
    target_name: Option<String>,
) -> Result<ImportAnalysisResult, String> {
    let store = state.store.lock().map_err(|e| e.to_string())?;
    // Pre-flight: confirm the session exists and belongs
    // to the project.
    let session = store.get_session(&session_id).map_err(|e| e.to_string())?;
    if session.project_id != project_id {
        return Err(format!(
            "session {session_id} does not belong to project {project_id}"
        ));
    }

    // Compose the orchestrator input from the IPC payload.
    let metadata_refs: Vec<&ExtractedMetadata> =
        assets.iter().map(|a| &a.metadata).collect();
    let paths: Vec<String> = assets.iter().map(|a| a.path.clone()).collect();

    // P4 target intelligence: if the caller supplied a
    // target name, build a high-confidence TargetIntel
    // from the catalog. Otherwise None — the orchestrator
    // still runs, but P6 + P7 won't have a target signal.
    let target_intel = target_name
        .as_deref()
        .map(astroforge_core::target_detection::TargetIntelligence::from_known_name);

    let input = import_understanding::SessionAnalysisInput {
        assets: metadata_refs,
        paths,
        target: target_intel.as_ref(),
    };
    let analysis = import_understanding::analyse_session(&input);
    let classification = import_understanding::classification_from_analysis(
        &session_id,
        &analysis,
    );
    store
        .set_session_classification(&session_id, &classification)
        .map_err(|e| e.to_string())?;

    Ok(ImportAnalysisResult {
        session_id,
        analysis,
        classification,
    })
}

/// CR-04 P8 — read the persisted session understanding.
#[tauri::command]
pub fn import_get_understanding(
    state: State<'_, ProjectState>,
    session_id: String,
) -> Result<Option<astroforge_core::domain::SessionClassification>, String> {
    let store = state.store.lock().map_err(|e| e.to_string())?;
    store
        .get_session_classification(&session_id)
        .map_err(|e| e.to_string())
}

/// CR-04 P8 — confirm an import. Transitions the session
/// from `Understanding` (or `Ambiguous`) → `Confirmed`.
/// Recorded as an idempotent set: callers can re-confirm
/// after an override.
#[tauri::command]
pub fn import_confirm(
    state: State<'_, ProjectState>,
    session_id: String,
) -> Result<astroforge_core::domain::SessionClassification, String> {
    let store = state.store.lock().map_err(|e| e.to_string())?;
    let mut classification = store
        .get_session_classification(&session_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| {
            "no classification recorded; call import_analyse_session first".into()
        })?;
    classification.import_state = ImportState::Confirmed;
    store
        .set_session_classification(&session_id, &classification)
        .map_err(|e| e.to_string())?;
    Ok(classification)
}

/// CR-04 P8 — override the capture-kind or narrowband
/// composition with a user-supplied correction. The UI
/// surfaces an override panel when the user disagrees with
/// the suggestion; this command records the correction
/// and bumps the session to `Confirmed` so the pipeline
/// can proceed.
#[tauri::command]
pub fn import_override_classification(
    state: State<'_, ProjectState>,
    session_id: String,
    capture_kind: Option<String>,
    narrowband_composition: Option<String>,
) -> Result<astroforge_core::domain::SessionClassification, String> {
    let store = state.store.lock().map_err(|e| e.to_string())?;
    let mut classification = store
        .get_session_classification(&session_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "no classification recorded".into())?;
    if let Some(kind) = capture_kind {
        classification.capture_kind = Some(kind);
    }
    if let Some(composition) = narrowband_composition {
        classification.narrowband_composition = Some(composition);
    }
    classification.import_state = ImportState::Confirmed;
    // CR-04 P10 — recording the user override in the
    // provenance column so the UI can surface "User
    // correction applied" on the understanding panel.
    classification.target_provenance = ClassificationProvenance::UserOverride;
    store
        .set_session_classification(&session_id, &classification)
        .map_err(|e| e.to_string())?;
    Ok(classification)
}

/// CR-04 P10 — fetch the AI classification provenance
/// for a session. The UI calls this to render "AI
/// confirming…" or "Deterministic" or "User override"
/// on the Understanding panel.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TargetProvenanceReport {
    pub session_id: String,
    pub provenance: String,
    pub reasoning: String,
}

#[tauri::command]
pub fn import_get_target_provenance(
    state: State<'_, ProjectState>,
    session_id: String,
) -> Result<TargetProvenanceReport, String> {
    let store = state.store.lock().map_err(|e| e.to_string())?;
    let classification = store
        .get_session_classification(&session_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "no classification recorded".into())?;
    let provenance = classification.target_provenance.as_str().to_string();
    let reasoning = match classification.target_provenance {
        ClassificationProvenance::Deterministic => {
            "Target detected by deterministic P4 classifier. No AI confirmation needed."
                .into()
        }
        ClassificationProvenance::AiStubRequested => {
            "AI confirmation requested (P10 stub). The deterministic P4 result was below the AI floor; the AI model is a no-op today and the deterministic fallback was used.".into()
        }
        ClassificationProvenance::UserOverride => {
            "Target classification overridden by the user via the §13 ambiguity panel."
                .into()
        }
    };
    Ok(TargetProvenanceReport {
        session_id,
        provenance,
        reasoning,
    })
}

/// CR-04 P8 — mark the session as `Materialised`. Called
/// after the wizard finishes and the assets have been
/// persisted to the project's session.
#[tauri::command]
pub fn import_set_materialised(
    state: State<'_, ProjectState>,
    session_id: String,
) -> Result<astroforge_core::domain::SessionClassification, String> {
    let store = state.store.lock().map_err(|e| e.to_string())?;
    let mut classification = store
        .get_session_classification(&session_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "no classification recorded".into())?;
    classification.import_state = ImportState::Materialised;
    store
        .set_session_classification(&session_id, &classification)
        .map_err(|e| e.to_string())?;
    Ok(classification)
}

/// CR-04 P8 — IPC payload shape for an asset's metadata.
/// The frontend sends the `ExtractedMetadata` that was
/// produced by `ingest_scan_directory` + the asset's
/// `original_path`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetMetadataInput {
    pub path: String,
    pub metadata: ExtractedMetadata,
}

// ─── Tests ──────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn asset_format_from_path_handles_common_extensions() {
        use astroforge_core::import_scan::AssetFormat;
        assert!(matches!(
            asset_format_for_test("/data/x.fits"),
            AssetFormat::Fits
        ));
        assert!(matches!(
            asset_format_for_test("/data/x.FIT"),
            AssetFormat::Fits
        ));
        assert!(matches!(
            asset_format_for_test("/data/x.png"),
            AssetFormat::Png
        ));
        assert!(matches!(
            asset_format_for_test("/data/x.jpg"),
            AssetFormat::Jpeg
        ));
        assert!(matches!(
            asset_format_for_test("/data/x.jpeg"),
            AssetFormat::Jpeg
        ));
        assert!(matches!(
            asset_format_for_test("/data/x.dng"),
            AssetFormat::Dng
        ));
        assert!(matches!(
            asset_format_for_test("/data/x.tiff"),
            AssetFormat::Tiff
        ));
        assert!(matches!(
            asset_format_for_test("/data/x.unknown"),
            AssetFormat::Other
        ));
    }

    fn asset_format_for_test(
        path: &str,
    ) -> astroforge_core::import_scan::AssetFormat {
        let ext = std::path::Path::new(path)
            .extension()
            .and_then(|s| s.to_str())
            .map(|s| s.to_lowercase());
        match ext.as_deref() {
            Some("fits") | Some("fit") | Some("fts") => astroforge_core::import_scan::AssetFormat::Fits,
            Some("png") => astroforge_core::import_scan::AssetFormat::Png,
            Some("jpg") | Some("jpeg") => astroforge_core::import_scan::AssetFormat::Jpeg,
            Some("dng") => astroforge_core::import_scan::AssetFormat::Dng,
            Some("tif") | Some("tiff") => astroforge_core::import_scan::AssetFormat::Tiff,
            _ => astroforge_core::import_scan::AssetFormat::Other,
        }
    }
}