//! CR-05 P4 slice 7 — §21 Resource-Aware Execution: expose the detected
//! `ResourceSnapshot` across the IPC boundary so the workspace can show
//! "AstroForge optimized processing for this device" plus the advanced
//! inspect block (CPU / GPU / Memory / Tile size / Precision / Backend).
//!
//! Stateless: the snapshot is recomputed on demand so it always reflects
//! current memory pressure, and nothing here touches the domain store.

use astroforge_core::resource::{
    BackendCapability, ExecutionBudget, ResourceSnapshot,
};

/// Detect the current device and return the snapshot with the derived
/// execution recommendation. Infallible by design — individual probes
/// degrade gracefully inside `ResourceSnapshot::detect()`.
#[tauri::command]
pub fn get_resource_snapshot() -> ResourceSnapshot {
    ResourceSnapshot::detect()
}

/// CR-05 P5 slice 1 (D-CR05-8) — advertised capability of every §21
/// backend on this device, including *why* a backend is unavailable so
/// Expert mode can render it greyed out with a reason instead of hiding it.
#[tauri::command]
pub fn list_backend_capabilities() -> Vec<BackendCapability> {
    astroforge_core::resource::enumerate_backends()
}

/// CR-05 P5 slice 1 (§22) — pre-flight budget for a stage whose input
/// dataset is `dataset_size_bytes` uncompressed. The UI renders
/// `warning` verbatim before expensive stages when memory is tight.
#[tauri::command]
pub fn derive_execution_budget(dataset_size_bytes: u64) -> ExecutionBudget {
    let snap = ResourceSnapshot::detect();
    astroforge_core::resource::derive_budget(
        dataset_size_bytes,
        snap.available_memory_bytes,
        snap.logical_cores,
    )
}

/// CR-05 P5 slice 2 (§22) — pre-flight budget read straight from a
/// stage's `parameters_json` (if the upstream handler stamped
/// `dataset_size_bytes`). Returns the same `ExecutionBudget` shape so
/// the UI can render the §22 copy verbatim without a separate code path.
#[tauri::command]
pub fn stage_execution_budget(parameters_json: Option<String>) -> ExecutionBudget {
    let snap = ResourceSnapshot::detect();
    let dataset_size_bytes = parameters_json
        .as_deref()
        .and_then(astroforge_core::resource::parse_dataset_size_bytes)
        .unwrap_or(astroforge_core::resource::UNKNOWN_DATASET_SIZE_BYTES);
    astroforge_core::resource::derive_stage_budget(&snap, dataset_size_bytes)
}
