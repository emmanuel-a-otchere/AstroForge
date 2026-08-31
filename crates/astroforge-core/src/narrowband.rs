use crate::image::F32Image;
use crate::ingest::LightGroup;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum NarrowbandFilter {
    Ha,
    OIII,
    SII,
    DualBand,
    Unknown,
}

impl NarrowbandFilter {
    pub fn from_filter_name(name: &str) -> Self {
        let upper = name.to_uppercase();
        if upper.contains("HA") || upper.contains("H-A") || upper.contains("Hα") {
            Self::Ha
        } else if upper.contains("OIII") || upper.contains("O-III") || upper.contains("O3") {
            Self::OIII
        } else if upper.contains("SII") || upper.contains("S-II") || upper.contains("S2") {
            Self::SII
        } else if upper.contains("DUAL") || upper.contains("DUO") {
            Self::DualBand
        } else {
            Self::Unknown
        }
    }

    pub fn primary_channel(&self) -> usize {
        match self {
            Self::Ha | Self::SII => 0,
            Self::OIII => 2,
            Self::DualBand => 0,
            Self::Unknown => 0,
        }
    }
}

pub fn detect_narrowband(groups: &[LightGroup]) -> Vec<(NarrowbandFilter, &LightGroup)> {
    let mut narrowband = Vec::new();
    for group in groups {
        let filter = NarrowbandFilter::from_filter_name(&group.filter);
        if filter != NarrowbandFilter::Unknown {
            narrowband.push((filter, group));
        }
    }
    narrowband
}

pub fn is_narrowband_session(groups: &[LightGroup]) -> bool {
    detect_narrowband(groups).len() >= 2
}

pub fn extract_channel(image: &F32Image, channel: usize) -> F32Image {
    let width = image.width();
    let height = image.height();
    let mut result = F32Image::new(width, height, 1);
    let ch = channel.min(image.channels() - 1);
    for y in 0..height {
        for x in 0..width {
            result[(0, y, x)] = image[(ch, y, x)];
        }
    }
    result
}

pub fn extract_oiii(image: &F32Image) -> F32Image {
    let width = image.width();
    let height = image.height();
    let mut result = F32Image::new(width, height, 1);
    for y in 0..height {
        for x in 0..width {
            let blue = if image.channels() > 2 { image[(2, y, x)] } else { 0.0 };
            let green = if image.channels() > 1 { image[(1, y, x)] } else { 0.0 };
            result[(0, y, x)] = (blue + green) / 2.0;
        }
    }
    result
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum CompositionPalette {
    HOO,
    SHO,
    HSO,
    Custom,
}

pub fn compose_palette(
    ha: Option<&F32Image>,
    sii: Option<&F32Image>,
    oiii: Option<&F32Image>,
    palette: CompositionPalette,
) -> F32Image {
    let width = ha.or(sii).or(oiii).map(|i| i.width()).unwrap_or(1);
    let height = ha.or(sii).or(oiii).map(|i| i.height()).unwrap_or(1);
    let mut result = F32Image::new(width, height, 3);

    let blank = F32Image::new(width, height, 1);

    let (r_src, g_src, b_src) = match palette {
        CompositionPalette::HOO => (ha, oiii, oiii),
        CompositionPalette::SHO => (sii, ha, oiii),
        CompositionPalette::HSO => (ha, sii, oiii),
        CompositionPalette::Custom => (ha, oiii, oiii),
    };

    let r = r_src.unwrap_or(&blank);
    let g = g_src.unwrap_or(&blank);
    let b = b_src.unwrap_or(&blank);

    for y in 0..height {
        for x in 0..width {
            result[(0, y, x)] = r[(0, y.min(r.height() - 1), x.min(r.width() - 1))];
            result[(1, y, x)] = g[(0, y.min(g.height() - 1), x.min(g.width() - 1))];
            result[(2, y, x)] = b[(0, y.min(b.height() - 1), x.min(b.width() - 1))];
        }
    }

    result
}

pub fn scnr_green(image: &F32Image, amount: f32) -> F32Image {
    let mut result = image.clone();
    if result.channels() < 3 {
        return result;
    }

    for y in 0..result.height() {
        for x in 0..result.width() {
            let r = result[(0, y, x)];
            let g = result[(1, y, x)];
            let b = result[(2, y, x)];
            let min_rb = r.min(b);
            if g > min_rb {
                let excess = g - min_rb;
                result[(1, y, x)] = g - excess * amount;
            }
        }
    }

    result
}

pub fn scnr_magenta(image: &F32Image, amount: f32) -> F32Image {
    let mut result = image.clone();
    if result.channels() < 3 {
        return result;
    }

    for y in 0..result.height() {
        for x in 0..result.width() {
            let r = result[(0, y, x)];
            let g = result[(1, y, x)];
            let b = result[(2, y, x)];
            let excess = r.max(b) - g;
            if excess > 0.0 {
                result[(0, y, x)] = r - excess * amount * 0.5;
                result[(2, y, x)] = b - excess * amount * 0.5;
            }
        }
    }

    result
}

pub fn normalize_channel_ratio(
    image: &F32Image,
    target_ratios: (f32, f32, f32),
) -> F32Image {
    let mut result = image.clone();
    if result.channels() < 3 {
        return result;
    }

    let r_mean = channel_mean(&result, 0).max(1e-10);
    let g_mean = channel_mean(&result, 1).max(1e-10);
    let b_mean = channel_mean(&result, 2).max(1e-10);

    let (target_r, target_g, target_b) = target_ratios;
    let target_sum = target_r + target_g + target_b;

    let r_gain = (target_r / target_sum) / (r_mean / (r_mean + g_mean + b_mean));
    let g_gain = (target_g / target_sum) / (g_mean / (r_mean + g_mean + b_mean));
    let b_gain = (target_b / target_sum) / (b_mean / (r_mean + g_mean + b_mean));

    for val in result.slice_mut(ndarray::s![0..1, .., ..]).iter_mut() {
        *val *= r_gain;
    }
    for val in result.slice_mut(ndarray::s![1..2, .., ..]).iter_mut() {
        *val *= g_gain;
    }
    for val in result.slice_mut(ndarray::s![2..3, .., ..]).iter_mut() {
        *val *= b_gain;
    }

    result
}

fn channel_mean(image: &F32Image, channel: usize) -> f32 {
    let slice = image.slice(ndarray::s![channel..channel + 1, .., ..]);
    let sum: f32 = slice.iter().sum();
    let count = slice.len();
    if count > 0 {
        sum / count as f32
    } else {
        0.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_channel_image(width: usize, height: usize, val: f32) -> F32Image {
        let mut img = F32Image::new(width, height, 1);
        img.fill(val);
        img
    }

    #[test]
    fn test_narrowband_filter_detection() {
        assert_eq!(NarrowbandFilter::from_filter_name("Ha"), NarrowbandFilter::Ha);
        assert_eq!(NarrowbandFilter::from_filter_name("OIII"), NarrowbandFilter::OIII);
        assert_eq!(NarrowbandFilter::from_filter_name("SII"), NarrowbandFilter::SII);
        assert_eq!(NarrowbandFilter::from_filter_name("L"), NarrowbandFilter::Unknown);
    }

    #[test]
    fn test_detect_narrowband() {
        let groups = vec![
            LightGroup { filter: "Ha".into(), binning: 1, frame_paths: vec![] },
            LightGroup { filter: "OIII".into(), binning: 1, frame_paths: vec![] },
            LightGroup { filter: "L".into(), binning: 1, frame_paths: vec![] },
        ];
        let narrowband = detect_narrowband(&groups);
        assert_eq!(narrowband.len(), 2);
    }

    #[test]
    fn test_is_narrowband_session() {
        let groups = vec![
            LightGroup { filter: "Ha".into(), binning: 1, frame_paths: vec![] },
            LightGroup { filter: "OIII".into(), binning: 1, frame_paths: vec![] },
        ];
        assert!(is_narrowband_session(&groups));

        let groups = vec![
            LightGroup { filter: "L".into(), binning: 1, frame_paths: vec![] },
        ];
        assert!(!is_narrowband_session(&groups));
    }

    #[test]
    fn test_extract_channel() {
        let mut img = F32Image::new(4, 4, 3);
        img[(0, 0, 0)] = 10.0;
        img[(1, 0, 0)] = 20.0;
        img[(2, 0, 0)] = 30.0;
        let ch1 = extract_channel(&img, 1);
        assert!((ch1[(0, 0, 0)] - 20.0).abs() < 0.01);
    }

    #[test]
    fn test_extract_oiii() {
        let mut img = F32Image::new(4, 4, 3);
        img[(1, 0, 0)] = 100.0;
        img[(2, 0, 0)] = 200.0;
        let oiii = extract_oiii(&img);
        assert!((oiii[(0, 0, 0)] - 150.0).abs() < 0.01);
    }

    #[test]
    fn test_compose_hoo() {
        let ha = make_channel_image(4, 4, 100.0);
        let oiii = make_channel_image(4, 4, 50.0);
        let result = compose_palette(Some(&ha), None, Some(&oiii), CompositionPalette::HOO);
        assert!((result[(0, 0, 0)] - 100.0).abs() < 0.01);
        assert!((result[(1, 0, 0)] - 50.0).abs() < 0.01);
        assert!((result[(2, 0, 0)] - 50.0).abs() < 0.01);
    }

    #[test]
    fn test_compose_sho() {
        let sii = make_channel_image(4, 4, 80.0);
        let ha = make_channel_image(4, 4, 100.0);
        let oiii = make_channel_image(4, 4, 50.0);
        let result = compose_palette(Some(&ha), Some(&sii), Some(&oiii), CompositionPalette::SHO);
        assert!((result[(0, 0, 0)] - 80.0).abs() < 0.01);
        assert!((result[(1, 0, 0)] - 100.0).abs() < 0.01);
        assert!((result[(2, 0, 0)] - 50.0).abs() < 0.01);
    }

    #[test]
    fn test_scnr_green() {
        let mut img = F32Image::new(4, 4, 3);
        img[(0, 0, 0)] = 50.0;
        img[(1, 0, 0)] = 200.0;
        img[(2, 0, 0)] = 50.0;
        let result = scnr_green(&img, 1.0);
        assert!(result[(1, 0, 0)] <= 50.0);
    }

    #[test]
    fn test_normalize_channel_ratio() {
        let mut img = F32Image::new(4, 4, 3);
        for y in 0..4 {
            for x in 0..4 {
                img[(0, y, x)] = 100.0;
                img[(1, y, x)] = 100.0;
                img[(2, y, x)] = 100.0;
            }
        }
        let result = normalize_channel_ratio(&img, (1.0, 1.0, 1.0));
        let r_mean = channel_mean(&result, 0);
        let g_mean = channel_mean(&result, 1);
        let b_mean = channel_mean(&result, 2);
        assert!((r_mean - g_mean).abs() < 1.0);
        assert!((g_mean - b_mean).abs() < 1.0);
    }
}
