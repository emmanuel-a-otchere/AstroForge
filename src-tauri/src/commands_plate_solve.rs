//! RT-12 (#430) — plate-solve annotation IPC (spike).
//!
//! The overlay path that works without the ASTAP binary: any FITS
//! carrying WCS keys in its header (solved upstream, or re-imported
//! from a solved export) yields a target + center + FOV badge for
//! the CR-07 canvas. Live solving stays behind the bundled-binary
//! path per ADR-0001's staged-adoption note — the solve IPC below
//! reports "not bundled" honestly rather than pretending.

use astroforge_core::fits::parse_header;
use astroforge_core::plate_solve::{overlay_from_header, AnnotationOverlay};
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlateSolveOverlayDto {
    /// Present when the file's header carries WCS keys; `None` is
    /// the honest "no WCS — no badge" answer.
    pub overlay: Option<AnnotationOverlay>,
}

/// Read a FITS artifact and extract the WCS-derived annotation
/// overlay from its header. Infallible apart from IO/parse errors:
/// a header without WCS keys returns `overlay: None`, never a
/// fabricated badge.
#[tauri::command]
pub fn plate_solve_overlay(path: String) -> Result<PlateSolveOverlayDto, String> {
    let bytes = std::fs::read(&path).map_err(|e| format!("read {path}: {e}"))?;
    // Some rows are header-only probes (the caller may pass a file
    // still being written); a parse failure is a real error.
    let header = parse_header(&bytes).map_err(|e| format!("parse FITS header: {e}"))?;
    let width = header.get_i64("NAXIS1").unwrap_or(0) as usize;
    let height = header.get_i64("NAXIS2").unwrap_or(0) as usize;
    Ok(PlateSolveOverlayDto {
        overlay: overlay_from_header(&header, width, height),
    })
}
