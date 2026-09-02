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
    Vec::new()
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
