//! RT-13 (#431) — session quality dashboard IPC.
//!
//! Computes the per-frame quality trends + outlier flags
//! (`astroforge_core::session_quality`) over a session's light-frame
//! paths, decoding each through the same FITS reader the pipeline
//! uses. The outlier list is the feed for the import recommendation
//! engine: a frame whose FWHM blows up mid-session is exactly the
//! frame the reviewer should consider dropping.

use astroforge_core::fits;
use astroforge_core::session_quality::{
    session_quality_report as build_report, FrameQuality, SessionQualityReport,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionQualityRequest {
    /// Absolute paths to the session's light frames, in capture order.
    pub frame_paths: Vec<String>,
}

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

/// Build the session quality report over the given frames. Decoding
/// failures are collected per-frame (never fatal): a truncated file
/// shows in `unreadable` instead of hiding the rest of the session.
#[tauri::command]
pub fn session_quality_report(
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
