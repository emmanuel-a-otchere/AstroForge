use crate::fits::{parse_header, FitsHeader};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum FrameType {
    Light,
    Dark,
    Flat,
    Bias,
}

impl FrameType {
    pub fn as_str(&self) -> &'static str {
        match self {
            FrameType::Light => "LIGHT",
            FrameType::Dark => "DARK",
            FrameType::Flat => "FLAT",
            FrameType::Bias => "BIAS",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrameInfo {
    pub path: PathBuf,
    pub frame_type: FrameType,
    pub exptime: Option<f64>,
    pub filter: Option<String>,
    pub date_obs: Option<String>,
    pub ccd_temp: Option<f64>,
    pub width: Option<i64>,
    pub height: Option<i64>,
    pub binning: Option<i64>,
    pub anomalies: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SessionManifest {
    pub session_id: String,
    pub source_dir: String,
    pub frames: Vec<FrameInfo>,
    pub light_groups: Vec<LightGroup>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LightGroup {
    pub filter: String,
    pub binning: i64,
    pub frame_paths: Vec<PathBuf>,
}

pub fn scan_directory(dir: &Path) -> Result<Vec<PathBuf>, std::io::Error> {
    let mut fits_files = Vec::new();
    scan_recursive(dir, &mut fits_files)?;
    Ok(fits_files)
}

fn scan_recursive(dir: &Path, files: &mut Vec<PathBuf>) -> Result<(), std::io::Error> {
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            scan_recursive(&path, files)?;
        } else if is_fits_file(&path) {
            files.push(path);
        }
    }
    Ok(())
}

fn is_fits_file(path: &Path) -> bool {
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
    matches!(ext.to_lowercase().as_str(), "fits" | "fit")
}

pub fn classify_frame(path: &Path, file_data: &[u8]) -> FrameInfo {
    let header = parse_header(file_data).ok();
    let frame_type = determine_frame_type(&header, path);
    let anomalies = detect_anomalies(&header);

    FrameInfo {
        path: path.to_path_buf(),
        frame_type,
        exptime: header.as_ref().and_then(|h| h.exptime()),
        filter: header.as_ref().and_then(|h| h.filter().map(|s| s.to_string())),
        date_obs: header.as_ref().and_then(|h| h.date_obs().map(|s| s.to_string())),
        ccd_temp: header.as_ref().and_then(|h| h.ccd_temp()),
        width: header.as_ref().and_then(|h| h.naxis1()),
        height: header.as_ref().and_then(|h| h.naxis2()),
        binning: header.as_ref().and_then(|h| h.get_i64("XBINNING")),
        anomalies,
    }
}

fn determine_frame_type(header: &Option<FitsHeader>, path: &Path) -> FrameType {
    if let Some(h) = header {
        if let Some(imagetyp) = h.imagetyp() {
            let typ = imagetyp.to_uppercase();
            if typ.contains("LIGHT") {
                return FrameType::Light;
            }
            if typ.contains("DARK") {
                return FrameType::Dark;
            }
            if typ.contains("FLAT") {
                return FrameType::Flat;
            }
            if typ.contains("BIAS") || typ.contains("OFFSET") {
                return FrameType::Bias;
            }
        }

        let exptime = h.exptime().unwrap_or(-1.0);
        if exptime == 0.0 {
            return FrameType::Bias;
        }
        if exptime > 0.0 && exptime < 1.0 {
            return FrameType::Dark;
        }
        if exptime > 0.0 {
            let filter = h.filter().unwrap_or("").to_uppercase();
            if filter.contains("FLAT") || path.to_string_lossy().to_lowercase().contains("flat") {
                return FrameType::Flat;
            }
            return FrameType::Light;
        }
    }

    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("").to_lowercase();
    if name.contains("dark") {
        FrameType::Dark
    } else if name.contains("flat") {
        FrameType::Flat
    } else if name.contains("bias") || name.contains("offset") {
        FrameType::Bias
    } else {
        FrameType::Light
    }
}

fn detect_anomalies(header: &Option<FitsHeader>) -> Vec<String> {
    let mut anomalies = Vec::new();
    if let Some(h) = header {
        if let Some(temp) = h.ccd_temp() {
            if temp > 20.0 {
                anomalies.push(format!("High CCD temperature: {:.1}C", temp));
            }
        }
        if let Some(exptime) = h.exptime() {
            if exptime < 0.0 {
                anomalies.push("Negative exposure time".into());
            }
        }
    }
    anomalies
}

pub fn group_lights(frames: &[FrameInfo]) -> Vec<LightGroup> {
    let mut groups: std::collections::HashMap<(String, i64), Vec<PathBuf>> = std::collections::HashMap::new();

    for frame in frames {
        if frame.frame_type != FrameType::Light {
            continue;
        }
        let filter = frame.filter.clone().unwrap_or_default();
        let binning = frame.binning.unwrap_or(1);
        groups
            .entry((filter, binning))
            .or_default()
            .push(frame.path.clone());
    }

    groups
        .into_iter()
        .map(|((filter, binning), paths)| LightGroup {
            filter,
            binning,
            frame_paths: paths,
        })
        .collect()
}

pub fn build_manifest(session_id: &str, source_dir: &str, frames: Vec<FrameInfo>) -> SessionManifest {
    let light_groups = group_lights(&frames);
    SessionManifest {
        session_id: session_id.to_string(),
        source_dir: source_dir.to_string(),
        frames,
        light_groups,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_determine_frame_type_from_header() {
        let mut header = FitsHeader::new();
        header.set("IMAGETYP", "LIGHT");
        assert_eq!(determine_frame_type(&Some(header), Path::new("test.fits")), FrameType::Light);

        let mut header = FitsHeader::new();
        header.set("IMAGETYP", "DARK");
        assert_eq!(determine_frame_type(&Some(header), Path::new("test.fits")), FrameType::Dark);

        let mut header = FitsHeader::new();
        header.set("IMAGETYP", "FLAT");
        assert_eq!(determine_frame_type(&Some(header), Path::new("test.fits")), FrameType::Flat);

        let mut header = FitsHeader::new();
        header.set("IMAGETYP", "BIAS");
        assert_eq!(determine_frame_type(&Some(header), Path::new("test.fits")), FrameType::Bias);
    }

    #[test]
    fn test_determine_frame_type_from_exptime() {
        let mut header = FitsHeader::new();
        header.set("EXPTIME", "0");
        assert_eq!(determine_frame_type(&Some(header), Path::new("test.fits")), FrameType::Bias);

        let mut header = FitsHeader::new();
        header.set("EXPTIME", "0.5");
        assert_eq!(determine_frame_type(&Some(header), Path::new("test.fits")), FrameType::Dark);

        let mut header = FitsHeader::new();
        header.set("EXPTIME", "120");
        assert_eq!(determine_frame_type(&Some(header), Path::new("test.fits")), FrameType::Light);
    }

    #[test]
    fn test_determine_frame_type_from_filename() {
        assert_eq!(determine_frame_type(&None, Path::new("dark_001.fits")), FrameType::Dark);
        assert_eq!(determine_frame_type(&None, Path::new("flat_001.fits")), FrameType::Flat);
        assert_eq!(determine_frame_type(&None, Path::new("bias_001.fits")), FrameType::Bias);
        assert_eq!(determine_frame_type(&None, Path::new("light_001.fits")), FrameType::Light);
    }

    #[test]
    fn test_group_lights_by_filter_and_binning() {
        let frames = vec![
            FrameInfo {
                path: PathBuf::from("light_ha_1.fits"),
                frame_type: FrameType::Light,
                exptime: Some(300.0),
                filter: Some("Ha".into()),
                date_obs: None,
                ccd_temp: None,
                width: None,
                height: None,
                binning: Some(1),
                anomalies: vec![],
            },
            FrameInfo {
                path: PathBuf::from("light_ha_2.fits"),
                frame_type: FrameType::Light,
                exptime: Some(300.0),
                filter: Some("Ha".into()),
                date_obs: None,
                ccd_temp: None,
                width: None,
                height: None,
                binning: Some(1),
                anomalies: vec![],
            },
            FrameInfo {
                path: PathBuf::from("light_oiii_1.fits"),
                frame_type: FrameType::Light,
                exptime: Some(300.0),
                filter: Some("OIII".into()),
                date_obs: None,
                ccd_temp: None,
                width: None,
                height: None,
                binning: Some(1),
                anomalies: vec![],
            },
            FrameInfo {
                path: PathBuf::from("dark_1.fits"),
                frame_type: FrameType::Dark,
                exptime: Some(300.0),
                filter: None,
                date_obs: None,
                ccd_temp: None,
                width: None,
                height: None,
                binning: Some(1),
                anomalies: vec![],
            },
        ];

        let groups = group_lights(&frames);
        assert_eq!(groups.len(), 2);
        let ha_group = groups.iter().find(|g| g.filter == "Ha").unwrap();
        assert_eq!(ha_group.frame_paths.len(), 2);
        let oiii_group = groups.iter().find(|g| g.filter == "OIII").unwrap();
        assert_eq!(oiii_group.frame_paths.len(), 1);
    }

    #[test]
    fn test_anomaly_detection() {
        let mut header = FitsHeader::new();
        header.set("CCD-TEMP", "25.0");
        let anomalies = detect_anomalies(&Some(header));
        assert!(!anomalies.is_empty());
        assert!(anomalies[0].contains("High CCD temperature"));
    }
}
