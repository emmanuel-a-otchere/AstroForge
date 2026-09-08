//! CR-05 P4 slice 7 — §21 Resource-Aware Execution: expose the detected
//! `ResourceSnapshot` across the IPC boundary so the workspace can show
//! "AstroForge optimized processing for this device" plus the advanced
//! inspect block (CPU / GPU / Memory / Tile size / Precision / Backend).
//!
//! Stateless: the snapshot is recomputed on demand so it always reflects
//! current memory pressure, and nothing here touches the domain store.

use astroforge_core::resource::ResourceSnapshot;

/// Detect the current device and return the snapshot with the derived
/// execution recommendation. Infallible by design — individual probes
/// degrade gracefully inside `ResourceSnapshot::detect()`.
#[tauri::command]
pub fn get_resource_snapshot() -> ResourceSnapshot {
    ResourceSnapshot::detect()
}
