//! Quality panels — port of two Tauri command groups to direct calls
//! over `Services`:
//! - `src-tauri/src/commands_session_quality.rs` (RT-13 #431):
//!   `session_quality_report` — per-frame quality trends + outlier
//!   flags over a session's light frames.
//! - `src-tauri/src/commands_plate_solve.rs` (RT-12 #430):
//!   `plate_solve_overlay` — WCS-derived annotation overlay read
//!   straight from a FITS header.
//!
//! Both commands are stateless in Tauri (they decode files directly),
//! so their bodies never read `services`; the parameter is kept so
//! every `api` function shares one call convention.

use serde::{Deserialize, Serialize};

use astroforge_core::fits;
use astroforge_core::fits::parse_header;
use astroforge_core::plate_solve::{overlay_from_header, AnnotationOverlay};
use astroforge_core::session_quality::{
    session_quality_report as build_report, FrameQuality, SessionQualityReport,
};

use crate::services::Services;

// ─── RT-13 (#431) — session quality dashboard (commands_session_quality.rs)

/// Request payload of `session_quality_report` (same shape the Tauri
/// command deserialized).
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionQualityRequest {
    /// Absolute paths to the session's light frames, in capture order.
    pub frame_paths: Vec<String>,
}

/// Response of `session_quality_report`.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionQualityReportDto {
    /// The core report verbatim (frames, trends, outliers).
    #[serde(flatten)]
    pub report: SessionQualityReport,
    /// Frames that failed to decode (path + reason) — surfaced so the
    /// dashboard stays honest about what it could not measure.
    pub unreadable: Vec<UnreadableFrameDto>,
}

#[derive(Debug, Serialize)]
pub struct UnreadableFrameDto {
    pub path: String,
    pub reason: String,
}

/// `session_quality_report` — build the session quality report over
/// the given frames. Decoding failures are collected per-frame (never
/// fatal): a truncated file shows in `unreadable` instead of hiding
/// the rest of the session.
#[allow(unused_variables)]
pub fn session_quality_report(
    services: &Services,
    request: SessionQualityRequest,
) -> Result<SessionQualityReportDto, String> {
    let mut frames = Vec::with_capacity(request.frame_paths.len());
    let mut unreadable = Vec::new();
    for (index, path) in request.frame_paths.iter().enumerate() {
        match std::fs::read(path) {
            Err(e) => unreadable.push(UnreadableFrameDto {
                path: path.clone(),
                reason: format!("read: {e}"),
            }),
            Ok(bytes) => match fits::parse_header(&bytes)
                .map_err(|e| e.to_string())
                .and_then(|h| fits::read_f32_image(&bytes, &h).map_err(|e| e.to_string()))
            {
                Ok(image) => {
                    let name = std::path::Path::new(path)
                        .file_name()
                        .map(|n| n.to_string_lossy().into_owned())
                        .unwrap_or_else(|| path.clone());
                    frames.push(FrameQuality::compute(index, &name, &image));
                }
                Err(e) => unreadable.push(UnreadableFrameDto {
                    path: path.clone(),
                    reason: format!("decode: {e}"),
                }),
            },
        }
    }
    Ok(SessionQualityReportDto {
        report: build_report(frames),
        unreadable,
    })
}

// ─── RT-12 (#430) — plate-solve annotation overlay (commands_plate_solve.rs)

/// Response of `plate_solve_overlay`.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlateSolveOverlayDto {
    /// Present when the file's header carries WCS keys; `None` is
    /// the honest "no WCS — no badge" answer.
    pub overlay: Option<AnnotationOverlay>,
}

/// `plate_solve_overlay` — read a FITS artifact and extract the
/// WCS-derived annotation overlay from its header. Infallible apart
/// from IO/parse errors: a header without WCS keys returns
/// `overlay: None`, never a fabricated badge.
#[allow(unused_variables)]
pub fn plate_solve_overlay(
    services: &Services,
    path: &str,
) -> Result<PlateSolveOverlayDto, String> {
    let bytes = std::fs::read(path).map_err(|e| format!("read {path}: {e}"))?;
    // Some rows are header-only probes (the caller may pass a file
    // still being written); a parse failure is a real error.
    let header = parse_header(&bytes).map_err(|e| format!("parse FITS header: {e}"))?;
    let width = header.get_i64("NAXIS1").unwrap_or(0) as usize;
    let height = header.get_i64("NAXIS2").unwrap_or(0) as usize;
    Ok(PlateSolveOverlayDto {
        overlay: overlay_from_header(&header, width, height),
    })
}
