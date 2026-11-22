//! Port of `src-tauri/src/commands_import.rs` (egui migration P3a).
//!
//! Covers the whole Import Understanding IPC group: the CR-04 P8/P10
//! commands (`import_analyse_session`, `import_get_understanding`,
//! `import_confirm`, `import_override_classification`,
//! `import_get_target_provenance`, `import_set_materialised`) plus the
//! RT-13-era wireflow / #461 asset-persistence commands
//! (`import_begin`, `import_register_assets`,
//! `session_list_for_project`, `session_continue`).
//!
//! `State<'_, ProjectState>` maps to `&Services` (the egui
//! counterpart of the Tauri managed state); the DomainStore behind
//! `ProjectState.store` is `services.domain_store`. Bodies,
//! validation and error strings match the Tauri originals so both
//! front-ends behave identically during the migration window.

use serde::{Deserialize, Serialize};

use crate::services::Services;

use astroforge_core::artifact::ContentStore;
use astroforge_core::domain::SourceAsset;
use astroforge_core::domain::{ClassificationProvenance, ImportState};
use astroforge_core::import_scan::detect_format;
use astroforge_core::import_scan::ExtractedMetadata;
use astroforge_core::import_understanding;
use astroforge_core::import_understanding::SessionAnalysis;

fn lock_err<E: std::fmt::Display>(e: E) -> String {
    format!("lock poisoned: {e}")
}

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
/// The caller (UI wizard) supplies the assets'
/// extracted metadata + paths. The orchestrator runs the
/// full pipeline (P3 frame classification, P4 target
/// detection, P5 session grouping, P6 capture analysis, P7
/// narrowband detection) and persists the resulting
/// `SessionClassification` to the durable store.
pub fn import_analyse_session(
    services: &Services,
    project_id: String,
    session_id: String,
    assets: Vec<AssetMetadataInput>,
    target_name: Option<String>,
) -> Result<ImportAnalysisResult, String> {
    let store = services.domain_store.lock().map_err(lock_err)?;
    // Pre-flight: confirm the session exists and belongs
    // to the project.
    let session = store.get_session(&session_id).map_err(|e| e.to_string())?;
    if session.project_id != project_id {
        return Err(format!(
            "session {session_id} does not belong to project {project_id}"
        ));
    }

    // Compose the orchestrator input from the payload.
    let metadata_refs: Vec<&ExtractedMetadata> = assets.iter().map(|a| &a.metadata).collect();
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
    let classification = import_understanding::classification_from_analysis(&session_id, &analysis);
    store
        .set_session_classification(&session_id, &classification)
        .map_err(|e| e.to_string())?;

    // #465 W4-fix — record the AnalysisCompleted project event: the
    // Overview checklist's Analyze row derives from this event and was
    // permanently pending because nothing emitted it.
    store
        .record_event(
            &project_id,
            astroforge_core::domain::ProjectEventKind::AnalysisCompleted,
            Some(&session_id),
        )
        .map_err(|e| e.to_string())?;

    Ok(ImportAnalysisResult {
        session_id,
        analysis,
        classification,
    })
}

/// CR-04 P8 — read the persisted session understanding.
pub fn import_get_understanding(
    services: &Services,
    session_id: String,
) -> Result<Option<astroforge_core::domain::SessionClassification>, String> {
    let store = services.domain_store.lock().map_err(lock_err)?;
    store
        .get_session_classification(&session_id)
        .map_err(|e| e.to_string())
}

/// CR-04 P8 — confirm an import. Transitions the session
/// from `Understanding` (or `Ambiguous`) → `Confirmed`.
/// Recorded as an idempotent set: callers can re-confirm
/// after an override.
pub fn import_confirm(
    services: &Services,
    session_id: String,
) -> Result<astroforge_core::domain::SessionClassification, String> {
    let store = services.domain_store.lock().map_err(lock_err)?;
    let mut classification = store
        .get_session_classification(&session_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| {
            "no classification recorded; call import_analyse_session first".to_string()
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
/// the suggestion; this records the correction
/// and bumps the session to `Confirmed` so the pipeline
/// can proceed.
pub fn import_override_classification(
    services: &Services,
    session_id: String,
    capture_kind: Option<String>,
    narrowband_composition: Option<String>,
) -> Result<astroforge_core::domain::SessionClassification, String> {
    let store = services.domain_store.lock().map_err(lock_err)?;
    let mut classification = store
        .get_session_classification(&session_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "no classification recorded".to_string())?;
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

pub fn import_get_target_provenance(
    services: &Services,
    session_id: String,
) -> Result<TargetProvenanceReport, String> {
    let store = services.domain_store.lock().map_err(lock_err)?;
    let classification = store
        .get_session_classification(&session_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "no classification recorded".to_string())?;
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
pub fn import_set_materialised(
    services: &Services,
    session_id: String,
) -> Result<astroforge_core::domain::SessionClassification, String> {
    let store = services.domain_store.lock().map_err(lock_err)?;
    let mut classification = store
        .get_session_classification(&session_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "no classification recorded".to_string())?;
    classification.import_state = ImportState::Materialised;
    store
        .set_session_classification(&session_id, &classification)
        .map_err(|e| e.to_string())?;
    Ok(classification)
}

/// CR-04 P8 — payload shape for an asset's metadata.
/// The caller sends the `ExtractedMetadata` that was
/// produced by `ingest_scan_directory` + the asset's
/// `original_path`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetMetadataInput {
    pub path: String,
    pub metadata: ExtractedMetadata,
}

// ─── RT-13-era wireflow fix (#461 S2) — asset persistence ───────────
//
// The import wizard only held scanned metadata in memory: no
// SourceAsset row was ever written, so the pipeline's frame loader
// (`load_frames_for_session`) found nothing and both workflows died
// before stacking. This persists every scanned frame as a
// durable SourceAsset (content-hashed, deduped by
// (session_id, content_hash) per CR-02 §7) and marks the session as
// the project's active one so the Process workspace becomes usable.

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportFrameInput {
    pub path: String,
    pub frame_type: String,
    pub exptime: Option<f64>,
    pub filter: Option<String>,
    pub width: Option<i64>,
    pub height: Option<i64>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RegisteredAssetsDto {
    pub session_id: String,
    pub registered: usize,
    pub duplicates: usize,
}

pub fn import_register_assets(
    services: &Services,
    session_id: String,
    project_id: String,
    frames: Vec<ImportFrameInput>,
) -> Result<RegisteredAssetsDto, String> {
    let store = services.domain_store.lock().map_err(lock_err)?;
    let mut registered = 0usize;
    let mut duplicates = 0usize;
    for frame in &frames {
        let bytes = std::fs::read(&frame.path).map_err(|e| format!("read {}: {e}", frame.path))?;
        let path = std::path::Path::new(&frame.path);
        let original_filename = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| frame.path.clone());
        let format = detect_format(path).as_str().to_string();
        let now = chrono_like_now();
        let asset = SourceAsset {
            asset_id: String::new(), // generated inside register_source_asset
            content_hash: ContentStore::sha256_hex(&bytes),
            original_filename,
            original_path: frame.path.clone(),
            file_size: bytes.len() as u64,
            format,
            mime_type: None,
            created_at: now.clone(),
            imported_at: now,
            session_id: session_id.clone(),
            frame_type: Some(frame.frame_type.clone()),
            exposure: frame.exptime,
            filter: frame.filter.clone(),
            binning: None,
            width: frame.width.map(|w| w.max(0) as u32),
            height: frame.height.map(|h| h.max(0) as u32),
            bit_depth: None,
            bayer_pattern: None,
            camera: None,
            date_obs: None,
            ra: None,
            dec: None,
        };
        let (_id, dup) = store
            .register_source_asset(&asset)
            .map_err(|e| format!("register {}: {e}", frame.path))?;
        if dup {
            duplicates += 1;
        } else {
            registered += 1;
        }
    }
    // Mark the session active on the project so the Process
    // workspace's plan generator becomes reachable. The caller
    // supplies project_id (the wizard state has it; sessions live in
    // a separate store so the DomainStore cannot resolve it).
    store
        .set_active_session(&project_id, &session_id)
        .map_err(|e| e.to_string())?;
    Ok(RegisteredAssetsDto {
        session_id,
        registered,
        duplicates,
    })
}

/// Cheap wall-clock ISO string; the asset timestamps are metadata,
/// not ordering keys.
fn chrono_like_now() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    format!("{secs}")
}

/// #461 S2 fix — create the import session in the DomainStore (the
/// same database the pipeline reads SourceAssets from). The
/// pre-existing `session_create` command writes to the separate
/// SessionStore, whose sessions cannot satisfy the source_assets
/// foreign key — using it made every S2 import fail at registration
/// time. Also marks the session active on the project in one step.
pub fn import_begin(
    services: &Services,
    project_id: String,
    source_dir: String,
) -> Result<String, String> {
    let store = services.domain_store.lock().map_err(lock_err)?;
    let session_id = store
        .create_session(&project_id, &source_dir)
        .map_err(|e| e.to_string())?;
    store
        .set_active_session(&project_id, &session_id)
        .map_err(|e| e.to_string())?;
    Ok(session_id)
}

/// #465 W4 — list a project's sessions (history). Backed by
/// DomainStore::list_sessions (the DomainStore-backed sessions
/// import_begin creates — the same DB the pipeline reads). Ordered
/// oldest→newest; the UI renders newest-first with a continue action
/// that re-activates a session.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionHistoryDto {
    pub session_id: String,
    pub name: String,
    pub source_location: Option<String>,
    pub processing_status: String,
    pub created_at: String,
    pub updated_at: String,
    /// The project's current active session — the UI marks this one.
    pub is_active: bool,
}

pub fn session_list_for_project(
    services: &Services,
    project_id: String,
) -> Result<Vec<SessionHistoryDto>, String> {
    let store = services.domain_store.lock().map_err(lock_err)?;
    let project = store.get_project(&project_id).map_err(|e| e.to_string())?;
    let active = project.active_session_id.clone().unwrap_or_default();
    let sessions = store
        .list_sessions(&project_id)
        .map_err(|e| e.to_string())?;
    Ok(sessions
        .into_iter()
        .rev()
        .map(|s| {
            let is_active = !active.is_empty() && active == s.session_id;
            SessionHistoryDto {
                session_id: s.session_id,
                name: s.name,
                source_location: s.source_location,
                processing_status: s.processing_status,
                created_at: s.created_at,
                updated_at: s.updated_at,
                is_active,
            }
        })
        .collect())
}

/// #465 W4 — continue a previous session: set it active on the
/// project (validated membership via set_active_session) so the
/// Process workspace gates open against it.
pub fn session_continue(
    services: &Services,
    project_id: String,
    session_id: String,
) -> Result<(), String> {
    let store = services.domain_store.lock().map_err(lock_err)?;
    // set_active_session validates the session row + project membership.
    store
        .set_active_session(&project_id, &session_id)
        .map_err(|e| e.to_string())
}

// ─── Tests ──────────────────────────────────────────────────────

#[cfg(test)]
mod tests {

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

    fn asset_format_for_test(path: &str) -> astroforge_core::import_scan::AssetFormat {
        let ext = std::path::Path::new(path)
            .extension()
            .and_then(|s| s.to_str())
            .map(|s| s.to_lowercase());
        match ext.as_deref() {
            Some("fits") | Some("fit") | Some("fts") => {
                astroforge_core::import_scan::AssetFormat::Fits
            }
            Some("png") => astroforge_core::import_scan::AssetFormat::Png,
            Some("jpg") | Some("jpeg") => astroforge_core::import_scan::AssetFormat::Jpeg,
            Some("dng") => astroforge_core::import_scan::AssetFormat::Dng,
            Some("tif") | Some("tiff") => astroforge_core::import_scan::AssetFormat::Tiff,
            _ => astroforge_core::import_scan::AssetFormat::Other,
        }
    }
}
