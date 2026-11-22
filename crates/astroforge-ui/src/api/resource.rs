//! CR-05 §21/§22 Resource-Aware Execution — port of
//! `src-tauri/src/commands_resource.rs` to direct calls over
//! `Services`.
//!
//! Stateless (the Tauri commands managed no state either): the
//! snapshot is recomputed on demand so it always reflects current
//! memory pressure, and nothing here touches the domain store. The
//! `services` parameter is kept so every `api` function shares one
//! call convention; these bodies never read it.

use astroforge_core::resource::{BackendCapability, ExecutionBudget, ResourceSnapshot};

use crate::services::Services;

/// `get_resource_snapshot` — detect the current device and return the
/// snapshot with the derived execution recommendation. Infallible by
/// design — individual probes degrade gracefully inside
/// `ResourceSnapshot::detect()`.
#[allow(unused_variables)]
pub fn get_resource_snapshot(services: &Services) -> Result<ResourceSnapshot, String> {
    Ok(ResourceSnapshot::detect())
}

/// `list_backend_capabilities` — CR-05 P5 slice 1 (D-CR05-8):
/// advertised capability of every §21 backend on this device,
/// including *why* a backend is unavailable so Expert mode can render
/// it greyed out with a reason instead of hiding it.
#[allow(unused_variables)]
pub fn list_backend_capabilities(services: &Services) -> Result<Vec<BackendCapability>, String> {
    Ok(astroforge_core::resource::enumerate_backends())
}

/// `derive_execution_budget` — CR-05 P5 slice 1 (§22): pre-flight
/// budget for a stage whose input dataset is `dataset_size_bytes`
/// uncompressed. The UI renders `warning` verbatim before expensive
/// stages when memory is tight.
#[allow(unused_variables)]
pub fn derive_execution_budget(
    services: &Services,
    dataset_size_bytes: u64,
) -> Result<ExecutionBudget, String> {
    let snap = ResourceSnapshot::detect();
    Ok(astroforge_core::resource::derive_budget(
        dataset_size_bytes,
        snap.available_memory_bytes,
        snap.logical_cores,
    ))
}

/// `stage_execution_budget` — CR-05 P5 slice 2 (§22): pre-flight
/// budget read straight from a stage's `parameters_json` (if the
/// upstream handler stamped `dataset_size_bytes`). Returns the same
/// `ExecutionBudget` shape so the UI can render the §22 copy verbatim
/// without a separate code path.
#[allow(unused_variables)]
pub fn stage_execution_budget(
    services: &Services,
    parameters_json: Option<String>,
) -> Result<ExecutionBudget, String> {
    let snap = ResourceSnapshot::detect();
    let dataset_size_bytes =
        astroforge_core::resource::parse_dataset_size_bytes(parameters_json.as_deref())
            .unwrap_or(astroforge_core::resource::UNKNOWN_DATASET_SIZE_BYTES);
    Ok(astroforge_core::resource::derive_stage_budget(
        &snap,
        dataset_size_bytes,
    ))
}
