use crate::fits::FitsHeader;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WcsSolution {
    pub ra_center: f64,
    pub dec_center: f64,
    pub field_width_arcmin: f64,
    pub field_height_arcmin: f64,
    pub rotation: f64,
    pub pixel_scale: f64,
    pub crpix1: f64,
    pub crpix2: f64,
    pub crval1: f64,
    pub crval2: f64,
    pub cd11: f64,
    pub cd12: f64,
    pub cd21: f64,
    pub cd22: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PlateSolveBackend {
    Astap,
    AstrometryNet,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlateSolveResult {
    pub solution: Option<WcsSolution>,
    pub backend: PlateSolveBackend,
    pub success: bool,
    pub error: Option<String>,
}

pub fn plate_solve(
    _image_data: &[u8],
    _focal_length_mm: f64,
    _pixel_size_um: f64,
    backend: PlateSolveBackend,
) -> PlateSolveResult {
    match backend {
        PlateSolveBackend::Astap => solve_with_astap(_image_data, _focal_length_mm, _pixel_size_um),
        PlateSolveBackend::AstrometryNet => {
            solve_with_astrometry(_image_data, _focal_length_mm, _pixel_size_um)
        }
    }
}

fn solve_with_astap(_image_data: &[u8], _focal_length: f64, _pixel_size: f64) -> PlateSolveResult {
    PlateSolveResult {
        solution: None,
        backend: PlateSolveBackend::Astap,
        success: false,
        error: Some("ASTAP binary not bundled in this build".into()),
    }
}

fn solve_with_astrometry(
    _image_data: &[u8],
    _focal_length: f64,
    _pixel_size: f64,
) -> PlateSolveResult {
    PlateSolveResult {
        solution: None,
        backend: PlateSolveBackend::AstrometryNet,
        success: false,
        error: Some("Online astrometry.net requires network and API key".into()),
    }
}

pub fn write_wcs_to_header(header: &mut FitsHeader, solution: &WcsSolution) {
    header.set("CTYPE1", "RA---TAN");
    header.set("CTYPE2", "DEC--TAN");
    header.set("CRPIX1", &solution.crpix1.to_string());
    header.set("CRPIX2", &solution.crpix2.to_string());
    header.set("CRVAL1", &solution.crval1.to_string());
    header.set("CRVAL2", &solution.crval2.to_string());
    header.set("CD1_1", &solution.cd11.to_string());
    header.set("CD1_2", &solution.cd12.to_string());
    header.set("CD2_1", &solution.cd21.to_string());
    header.set("CD2_2", &solution.cd22.to_string());
    header.set("RA", &solution.ra_center.to_string());
    header.set("DEC", &solution.dec_center.to_string());
}

pub fn auto_crop_with_wcs(
    image_width: usize,
    image_height: usize,
    solution: &WcsSolution,
    subject_ra: f64,
    subject_dec: f64,
    subject_width_arcmin: f64,
    subject_height_arcmin: f64,
) -> Option<crate::crop::CropRegion> {
    let pixel_scale = solution.pixel_scale.max(1e-10);
    let center_x = image_width as f64 / 2.0;
    let center_y = image_height as f64 / 2.0;

    let ra_offset = (subject_ra - solution.ra_center) * 3600.0;
    let dec_offset = (subject_dec - solution.dec_center) * 3600.0;

    let subject_center_x = center_x + ra_offset / pixel_scale;
    let subject_center_y = center_y + dec_offset / pixel_scale;

    let crop_w = (subject_width_arcmin * 60.0 / pixel_scale) as usize;
    let crop_h = (subject_height_arcmin * 60.0 / pixel_scale) as usize;

    if crop_w == 0 || crop_h == 0 || crop_w > image_width || crop_h > image_height {
        return None;
    }

    let x = (subject_center_x - crop_w as f64 / 2.0).max(0.0) as usize;
    let y = (subject_center_y - crop_h as f64 / 2.0).max(0.0) as usize;

    Some(crate::crop::CropRegion {
        x: x.min(image_width - crop_w),
        y: y.min(image_height - crop_h),
        width: crop_w,
        height: crop_h,
    })
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StarAnnotation {
    pub x: f64,
    pub y: f64,
    pub name: String,
    pub magnitude: f64,
}

pub fn annotate_stars(
    _solution: &WcsSolution,
    _image_width: usize,
    _image_height: usize,
) -> Vec<StarAnnotation> {
    // Named-star annotation needs a bundled star catalog (ASTAP H17,
    // ADR-0001). Until the binary + catalog ship together in v1.x
    // packaging, this stays empty by design rather than guessing
    // magnitudes from a catalog we do not carry.
    Vec::new()
}

// ─── RT-12 (#430) — WCS-derived annotation overlay ─────────────────
//
// The annotation path that works TODAY: any FITS carrying WCS keys
// (solved upstream, or re-imported from a solved export) renders a
// target + center + FOV overlay without the ASTAP binary. Live
// solving stays behind the bundled-binary path per ADR-0001's staged
// adoption note.

/// The UI payload for a WCS annotation overlay. `source` records
/// where the WCS came from so the badge stays honest.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AnnotationOverlay {
    pub target_name: String,
    pub ra_center: String,
    pub dec_center: String,
    pub field_width_arcmin: f64,
    pub field_height_arcmin: f64,
    pub rotation_deg: f64,
    pub source: WcsSource,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum WcsSource {
    /// Read from the file's own FITS header (CRVAL/CD keys).
    Header,
    /// Produced by a live solve (ASTAP; not bundled in v0.x).
    Solved,
}

/// Extract an overlay from WCS keys already present in a FITS
/// header: OBJECT for the name, CRVAL1/2 for the center, CD1_1 /
/// CD2_2 for the pixel scale (CDELT1/2 fallback), NAXIS1/2 for the
/// field. Returns `None` when the header carries no WCS — the UI
/// then shows no badge rather than a fabricated one.
pub fn overlay_from_header(
    header: &FitsHeader,
    width: usize,
    height: usize,
) -> Option<AnnotationOverlay> {
    let crval1 = header.get_f64("CRVAL1")?;
    let crval2 = header.get_f64("CRVAL2")?;
    // Pixel scale: CD matrix (arcsec/pixel) with CDELT fallback
    // (degrees/pixel, sign-flipped convention aside — magnitude is
    // what the FOV needs).
    let scale_arcsec = header
        .get_f64("CD1_1")
        .or_else(|| header.get_f64("CDELT1").map(|d| d * 3600.0))
        .unwrap_or(0.0);
    let (w, h) = if width > 0 && height > 0 {
        (width as f64, height as f64)
    } else {
        (
            header.get_i64("NAXIS1").unwrap_or(0) as f64,
            header.get_i64("NAXIS2").unwrap_or(0) as f64,
        )
    };
    // FITS string values arrive with their quotes ('M42'); the UI
    // wants the bare name.
    let target = header
        .get("OBJECT")
        .map(|v| v.trim_matches('\'').to_string())
        .unwrap_or_default();
    Some(AnnotationOverlay {
        target_name: target,
        ra_center: format_ra_hms(crval1),
        dec_center: format_dec_dms(crval2),
        field_width_arcmin: w * scale_arcsec / 60.0,
        field_height_arcmin: h * scale_arcsec / 60.0,
        rotation_deg: header.get_f64("CROTA1").unwrap_or(0.0),
        source: WcsSource::Header,
    })
}

/// Format RA (degrees) as `HHh MMm SS.s`.
pub fn format_ra_hms(ra_deg: f64) -> String {
    let ra = ra_deg.rem_euclid(360.0);
    let hours = ra / 15.0;
    let h = hours.floor();
    let m_frac = (hours - h) * 60.0;
    let m = m_frac.floor();
    let sec = (m_frac - m) * 60.0;
    format!("{h:02}h {m:02}m {sec:04.1}s")
}

/// Format Dec (degrees) as `+DD° MM' SS"`.
pub fn format_dec_dms(dec_deg: f64) -> String {
    let sign = if dec_deg < 0.0 { '-' } else { '+' };
    let dec = dec_deg.abs();
    let d = dec.floor();
    let m_frac = (dec - d) * 60.0;
    let m = m_frac.floor();
    let sec = (m_frac - m) * 60.0;
    format!("{sign}{d:02}° {m:02}' {sec:04.1}\"")
}

pub fn handle_solve_failure(result: &PlateSolveResult) -> bool {
    !result.success
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plate_solve_astap_not_bundled() {
        let result = plate_solve(&[], 250.0, 2.9, PlateSolveBackend::Astap);
        assert!(!result.success);
        assert!(result.error.is_some());
    }

    #[test]
    fn ra_dec_formatting_round_trips() {
        assert_eq!(format_ra_hms(0.0), "00h 00m 00.0s");
        // 5h30m = 82.5 deg
        assert_eq!(format_ra_hms(82.5), "05h 30m 00.0s");
        // M42 center ≈ 05h35m, -05°23'
        assert!(format_ra_hms(83.82).starts_with("05h 35m"));
        assert!(format_dec_dms(-5.39).starts_with("-05° 23'"));
        assert_eq!(format_dec_dms(0.0), "+00° 00' 00.0\"");
        // Wrap-around: 360 deg -> 0h
        assert_eq!(format_ra_hms(360.0), "00h 00m 00.0s");
    }

    #[test]
    fn overlay_from_header_reads_wcs_keys() {
        let mut header = FitsHeader::new();
        header.set("OBJECT", "'M42 Sample'");
        header.set("CRVAL1", "83.82");
        header.set("CRVAL2", "-5.39");
        header.set("CD1_1", "2.4"); // arcsec/pixel
        header.set("NAXIS1", "256");
        header.set("NAXIS2", "256");
        let overlay = overlay_from_header(&header, 256, 256).expect("wcs header yields overlay");
        assert_eq!(overlay.target_name, "M42 Sample");
        assert!(overlay.ra_center.starts_with("05h 35m"));
        assert!(overlay.dec_center.starts_with("-05° 23'"));
        // 256 px * 2.4 arcsec = 614.4 arcsec = 10.24 arcmin
        assert!((overlay.field_width_arcmin - 10.24).abs() < 1e-9);
        assert_eq!(overlay.source, WcsSource::Header);
    }

    #[test]
    fn overlay_from_header_without_wcs_is_none() {
        let mut header = FitsHeader::new();
        header.set("OBJECT", "'No WCS here'");
        assert!(overlay_from_header(&header, 64, 64).is_none());
    }

    #[test]
    fn test_plate_solve_astrometry_needs_network() {
        let result = plate_solve(&[], 250.0, 2.9, PlateSolveBackend::AstrometryNet);
        assert!(!result.success);
        assert!(result.error.is_some());
    }

    #[test]
    fn test_write_wcs_to_header() {
        let mut header = FitsHeader::new();
        let solution = WcsSolution {
            ra_center: 83.633,
            dec_center: 22.014,
            field_width_arcmin: 60.0,
            field_height_arcmin: 40.0,
            rotation: 0.0,
            pixel_scale: 1.5,
            crpix1: 512.0,
            crpix2: 512.0,
            crval1: 83.633,
            crval2: 22.014,
            cd11: -0.026,
            cd12: 0.0,
            cd21: 0.0,
            cd22: 0.026,
        };
        write_wcs_to_header(&mut header, &solution);
        assert_eq!(header.get("CTYPE1"), Some("RA---TAN"));
        assert_eq!(header.get("CRPIX1"), Some("512"));
    }

    #[test]
    fn test_auto_crop_with_wcs() {
        let solution = WcsSolution {
            ra_center: 83.633,
            dec_center: 22.014,
            field_width_arcmin: 60.0,
            field_height_arcmin: 40.0,
            rotation: 0.0,
            pixel_scale: 1.5,
            crpix1: 512.0,
            crpix2: 512.0,
            crval1: 83.633,
            crval2: 22.014,
            cd11: -0.026,
            cd12: 0.0,
            cd21: 0.0,
            cd22: 0.026,
        };
        let region = auto_crop_with_wcs(2048, 2048, &solution, 83.633, 22.014, 30.0, 20.0);
        assert!(region.is_some());
        let r = region.unwrap();
        assert!(r.width > 0 && r.height > 0);
    }

    #[test]
    fn test_handle_solve_failure() {
        let result = PlateSolveResult {
            solution: None,
            backend: PlateSolveBackend::Astap,
            success: false,
            error: Some("failed".into()),
        };
        assert!(handle_solve_failure(&result));
    }
}
