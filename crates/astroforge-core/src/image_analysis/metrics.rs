//! CR-06 P2 — image-characteristics metrics.
//!
//! Pure functions on `F32Image`. Each returns a
//! `MetricsSample` so the report layer can attach the
//! sampled value to the observation with confidence.
//!
//! Strategies are heuristic and bounded by O(W·H). The
//! metrics stay cheap so P2's analyzer fits inside the
//! 4–8 GB target system's preview budget; ML-driven
//! characterization lands in a later phase if needed.

use serde::{Deserialize, Serialize};

use crate::image::F32Image;

/// How confident the analyzer is in a measurement. The
/// confidence score is the report's trust signal; the UI
/// renders it next to every observation per CR-06 §5.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Confidence {
    /// Single-hypothesis heuristic, no robustness check.
    Low,
    /// Cross-checked across sub-regions or via two metrics
    /// that agree.
    Medium,
    /// Multiple independent signals agree within tolerance.
    High,
}

impl Confidence {
    pub fn as_score(self) -> f32 {
        match self {
            Confidence::Low => 0.5,
            Confidence::Medium => 0.75,
            Confidence::High => 0.92,
        }
    }
}

/// One observation, with a sampled value, an optional
/// supporting region (the "evidence"), and a confidence
/// level.
///
/// CR-06 §5.1 — every report observation has supporting
/// evidence. The report layer attaches the relevant
/// evidence so the UI can render "Noise: HIGH (94%
/// confidence)" rather than an opaque score.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MetricsSample {
    pub value: f64,
    /// Optional human-readable label (e.g. "background
    /// gradient slope per 100px").
    pub label: Option<String>,
    /// Optional supporting region — a `Vec<u32>` interpreted
    /// as `[x, y, w, h]` (pixels) so the UI can highlight
    /// where the measurement came from.
    pub evidence_region: Option<[u32; 4]>,
    pub confidence: Confidence,
}

/// CR-07 §8.3: Color gradient: per-channel background gradient.
///
/// Computes the background gradient magnitude (same algorithm
/// as `background_gradient`) for each channel of the image and
/// returns the maximum across channels as the scalar metric.
///
/// A flat-field or uniform gradient (vignetting, light pollution)
/// produces a similar gradient magnitude across channels; a
/// strongly color-dependent gradient (e.g. red light pollution
/// dominating, blue channel near-uniform) produces a higher
/// maximum because one channel's gradient will exceed the
/// others.
///
/// Returns a zero-value `MetricsSample` with `Confidence::Low`
/// when the image is too small to tile (< 64×64) or when the
/// channel count is < 2 (single-channel images have no color
/// gradient by definition).
pub fn color_gradient(image: &F32Image) -> MetricsSample {
    let (w, h) = (image.width(), image.height());
    let channels = image.channels();
    if w < 64 || h < 64 || channels < 2 {
        return MetricsSample {
            value: 0.0,
            label: Some("color gradient (max per-channel magnitude per 100 px)".into()),
            evidence_region: None,
            confidence: Confidence::Low,
        };
    }
    let tile = 64usize;
    let mut max_mag = 0.0f64;
    for ch in 0..channels {
        let mut xs = Vec::new();
        let mut ys = Vec::new();
        let mut vs = Vec::new();
        let mut y0 = 0;
        while y0 < h {
            let mut x0 = 0;
            while x0 < w {
                let x1 = (x0 + tile).min(w);
                let y1 = (y0 + tile).min(h);
                let mut samples = Vec::with_capacity(16);
                for sy in (y0..y1).step_by((y1 - y0).max(1) / 4 + 1) {
                    for sx in (x0..x1).step_by((x1 - x0).max(1) / 4 + 1) {
                        if sx < x1 && sy < y1 {
                            samples.push(image[(ch, sy, sx)] as f64);
                        }
                    }
                }
                if !samples.is_empty() {
                    samples.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
                    let med = samples[samples.len() / 2];
                    xs.push((x0 as f64 + x1 as f64) / 2.0);
                    ys.push((y0 as f64 + y1 as f64) / 2.0);
                    vs.push(med);
                }
                x0 += tile;
            }
            y0 += tile;
        }
        if vs.len() < 2 {
            continue;
        }
        // Fit plane z = a*x + b*y + c via least-squares.
        let n = vs.len() as f64;
        let sx = xs.iter().sum::<f64>() / n;
        let sy = ys.iter().sum::<f64>() / n;
        let sz = vs.iter().sum::<f64>() / n;
        let sxx = xs.iter().map(|x| x * x).sum::<f64>() / n;
        let sxy = xs.iter().zip(ys.iter()).map(|(x, y)| x * y).sum::<f64>() / n;
        let sxz = xs.iter().zip(vs.iter()).map(|(x, v)| x * v).sum::<f64>() / n;
        let syy = ys.iter().map(|y| y * y).sum::<f64>() / n;
        let syz = ys.iter().zip(vs.iter()).map(|(y, v)| y * v).sum::<f64>() / n;
        let denom = (sxx - sx * sx) * (syy - sy * sy) - (sxy - sx * sy).powi(2);
        if denom.abs() < 1e-12 {
            continue;
        }
        let a = ((syy - sy * sy) * (sxz - sx * sz) - (sxy - sx * sy) * (syz - sy * sz)) / denom;
        let b = ((sxx - sx * sx) * (syz - sy * sz) - (sxy - sx * sy) * (sxz - sx * sz)) / denom;
        let mag_per_px = (a * a + b * b).sqrt();
        let value = mag_per_px * 100.0;
        if value > max_mag {
            max_mag = value;
        }
    }
    MetricsSample {
        value: max_mag,
        label: Some("color gradient (max per-channel magnitude per 100 px)".into()),
        evidence_region: Some([0, 0, w as u32, h as u32]),
        confidence: if channels >= 3 {
            Confidence::High
        } else {
            Confidence::Medium
        },
    }
}

/// CR-07 §8.2: Edge response: Sobel edge magnitude.
///
/// For each pixel in the preview, computes the Sobel
/// gradient magnitude `sqrt(Gx² + Gy²)` where `Gx` is the
/// horizontal Sobel kernel and `Gy` is the vertical Sobel
/// kernel applied to the 3×3 neighbourhood. Returns the
/// mean magnitude across the preview as a `MetricsSample`.
///
/// Edge response is a proxy for image sharpness / focus
/// quality: a sharp image has many high-magnitude edges
/// (star outlines, nebular structure); a soft image has
/// low-magnitude edges everywhere. The metric is bounded
/// by `O(W·H)` and runs on the downsampled preview
/// (max 64×64 pixels) so it fits the per-metric budget.
pub fn edge_response(image: &F32Image) -> MetricsSample {
    let (w, h) = (image.width(), image.height());
    if w < 3 || h < 3 {
        return MetricsSample {
            value: 0.0,
            label: Some("edge response (Sobel magnitude mean)".into()),
            evidence_region: None,
            confidence: Confidence::Low,
        };
    }
    let preview = image.downsample_box(0.25);
    let (pw, ph) = (preview.width(), preview.height());
    if pw < 3 || ph < 3 {
        return MetricsSample {
            value: 0.0,
            label: Some("edge response (Sobel magnitude mean)".into()),
            evidence_region: None,
            confidence: Confidence::Low,
        };
    }
    let mut sum = 0.0f64;
    let mut n = 0usize;
    for y in 1..ph - 1 {
        for x in 1..pw - 1 {
            let p00 = preview[(0, y - 1, x - 1)] as f64;
            let p01 = preview[(0, y - 1, x)] as f64;
            let p02 = preview[(0, y - 1, x + 1)] as f64;
            let p10 = preview[(0, y, x - 1)] as f64;
            let p12 = preview[(0, y, x + 1)] as f64;
            let p20 = preview[(0, y + 1, x - 1)] as f64;
            let p21 = preview[(0, y + 1, x)] as f64;
            let p22 = preview[(0, y + 1, x + 1)] as f64;
            // Sobel kernels:
            //   Gx = [[-1, 0, 1], [-2, 0, 2], [-1, 0, 1]]
            //   Gy = [[-1, -2, -1], [0, 0, 0], [1, 2, 1]]
            let gx = -p00 + p02 - 2.0 * p10 + 2.0 * p12 - p20 + p22;
            let gy = -p00 - 2.0 * p01 - p02 + p20 + 2.0 * p21 + p22;
            let mag = (gx * gx + gy * gy).sqrt();
            sum += mag;
            n += 1;
        }
    }
    let mean = if n > 0 { sum / n as f64 } else { 0.0 };
    MetricsSample {
        value: mean,
        label: Some("edge response (Sobel magnitude mean)".into()),
        evidence_region: Some([0, 0, w as u32, h as u32]),
        confidence: if n >= 4096 {
            Confidence::High
        } else if n >= 1024 {
            Confidence::Medium
        } else {
            Confidence::Low
        },
    }
}

/// CR-07 §8.1: Regional noise: spatial noise variation metric.
///
/// Splits the image into a `GRID × GRID` grid of tiles and
/// computes the luminance-noise sigma (MAD-based, same algorithm
/// as `luminance_noise`) within each tile. Returns a
/// `MetricsSample` whose `value` is the coefficient of variation
/// (CV = stddev / mean) of the per-tile sigmas: a low CV means
/// the noise is spatially uniform; a high CV means the image has
/// spatially-varying noise (e.g. amp glow in corners, vignetting
/// noise patterns).
///
/// The scalar metric is what the `noise.regional` registry key
/// expects (a single `f64` for the delta table). The full
/// per-tile map is available via `regional_noise_map()` for the
/// UI's spatial noise display.
///
/// Returns a zero-value `MetricsSample` with `Confidence::Low`
/// when the image is too small to tile (smaller than `GRID`
/// tiles of at least 8 pixels each).
pub fn regional_noise(image: &F32Image) -> MetricsSample {
    let tiles = regional_noise_map(image);
    if tiles.is_empty() {
        return MetricsSample {
            value: 0.0,
            label: Some("regional noise CV".into()),
            evidence_region: None,
            confidence: Confidence::Low,
        };
    }
    let sigmas: Vec<f64> = tiles.iter().map(|t| t.sigma).collect();
    let mean = sigmas.iter().sum::<f64>() / sigmas.len() as f64;
    if mean <= 0.0 {
        return MetricsSample {
            value: 0.0,
            label: Some("regional noise CV".into()),
            evidence_region: None,
            confidence: Confidence::Low,
        };
    }
    let variance = sigmas.iter().map(|s| (s - mean).powi(2)).sum::<f64>() / sigmas.len() as f64;
    let stddev = variance.sqrt();
    let cv = stddev / mean;
    MetricsSample {
        value: cv,
        label: Some("regional noise CV (stddev/mean across tiles)".into()),
        evidence_region: Some([0, 0, image.width() as u32, image.height() as u32]),
        confidence: if tiles.len() >= 12 {
            Confidence::High
        } else if tiles.len() >= 6 {
            Confidence::Medium
        } else {
            Confidence::Low
        },
    }
}

/// CR-07 §8.1: Regional noise map: per-tile noise sigma.
///
/// Splits the image into a `GRID × GRID` grid of tiles and
/// computes the luminance-noise sigma (MAD-based, same algorithm
/// as `luminance_noise`) within each tile. Returns a vector of
/// `RegionalNoiseSample` so the UI can render a spatial noise
/// map (e.g. center vs. corners).
///
/// The algorithm is identical to `luminance_noise` but applied
/// per-tile rather than globally. Each tile is downsampled to
/// at most 64×64 pixels before the 3×3 median filter, so the
/// per-tile cost is bounded by `O(64²·9)` and the total cost
/// is `O(W·H)` as with `luminance_noise`.
///
/// Returns an empty vector when the image is too small to tile
/// (smaller than `GRID` tiles of at least 8 pixels each).
pub fn regional_noise_map(image: &F32Image) -> Vec<RegionalNoiseSample> {
    const GRID: usize = 4;
    const MIN_TILE: usize = 8;

    let (w, h) = (image.width(), image.height());
    if w < GRID * MIN_TILE || h < GRID * MIN_TILE {
        return Vec::new();
    }

    let tile_w = w / GRID;
    let tile_h = h / GRID;
    let mut samples = Vec::with_capacity(GRID * GRID);

    for ty in 0..GRID {
        for tx in 0..GRID {
            let x0 = tx * tile_w;
            let y0 = ty * tile_h;
            let x1 = if tx == GRID - 1 { w } else { x0 + tile_w };
            let y1 = if ty == GRID - 1 { h } else { y0 + tile_h };
            let tw = x1 - x0;
            let th = y1 - y0;
            if tw < 3 || th < 3 {
                continue;
            }

            // Downsample the tile to at most 64×64 so the
            // 3×3 median filter stays cheap. The downsample
            // factor is chosen to keep the tile under 64×64
            // while preserving aspect ratio.
            let max_dim = tw.max(th);
            let factor = if max_dim > 64 {
                64.0 / max_dim as f64
            } else {
                1.0
            };
            let pw = ((tw as f64) * factor).max(1.0) as usize;
            let ph = ((th as f64) * factor).max(1.0) as usize;
            if pw < 3 || ph < 3 {
                continue;
            }

            // Build a downsampled tile buffer.
            let mut tile_data = vec![0.0f32; pw * ph];
            for y in 0..ph {
                for x in 0..pw {
                    let sx = x0 + ((x as f64) / factor) as usize;
                    let sy = y0 + ((y as f64) / factor) as usize;
                    let sx = sx.min(x1 - 1);
                    let sy = sy.min(y1 - 1);
                    tile_data[y * pw + x] = image[(0, sy, sx)];
                }
            }

            // Compute residuals (pixel minus 3×3 median).
            let mut residuals = Vec::with_capacity(pw * ph);
            for y in 1..ph - 1 {
                for x in 1..pw - 1 {
                    let v = tile_data[y * pw + x] as f64;
                    let mut window = [0.0f64; 9];
                    let mut wi = 0;
                    for dy in -1i32..=1 {
                        for dx in -1i32..=1 {
                            let yy = (y as i32 + dy) as usize;
                            let xx = (x as i32 + dx) as usize;
                            window[wi] = tile_data[yy * pw + xx] as f64;
                            wi += 1;
                        }
                    }
                    window.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
                    let median = window[4];
                    residuals.push((v - median).abs());
                }
            }
            if residuals.is_empty() {
                continue;
            }

            // Robust sigma via MAD.
            residuals.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
            let mad = residuals[residuals.len() / 2];
            let sigma = 1.4826 * mad;

            let confidence = if residuals.len() >= 256 {
                Confidence::High
            } else if residuals.len() >= 64 {
                Confidence::Medium
            } else {
                Confidence::Low
            };

            samples.push(RegionalNoiseSample {
                region: [x0 as u32, y0 as u32, tw as u32, th as u32],
                sigma,
                confidence,
            });
        }
    }

    samples
}

/// CR-07 §8.1: one tile of the regional noise map.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RegionalNoiseSample {
    /// Tile bounds as `[x, y, width, height]` in pixels.
    pub region: [u32; 4],
    /// Noise sigma in pixel units (same scale as
    /// `luminance_noise`).
    pub sigma: f64,
    /// Confidence in the tile's sigma estimate.
    pub confidence: Confidence,
}

/// CR-06 §5.1 — Noise level on the luminance channel,
/// binned as `Low / Moderate / High / Extreme` based on
/// sigma-clipping over the image's residuals (pixel value
/// minus local median).
///
/// Returns the noise sigma in pixel units plus a
/// confidence score. The local median is computed over a
/// fixed-size window so the metric is `O(W·H)` and fits
/// the preview budget.
pub fn luminance_noise(image: &F32Image) -> MetricsSample {
    let (w, h) = (image.width(), image.height());
    if w == 0 || h == 0 {
        return MetricsSample {
            value: 0.0,
            label: Some("luminance noise sigma".into()),
            evidence_region: None,
            confidence: Confidence::Low,
        };
    }
    // Downsample to a workable size for the per-pixel
    // residual. The full-resolution pass would be `O(W·H·9)`
    // for the 3x3 median; the preview pass runs on at most
    // 256×256 = 65 536 pixels.
    let preview = image.downsample_box(0.25);
    let (pw, ph) = (preview.width(), preview.height());
    if pw < 3 || ph < 3 {
        return MetricsSample {
            value: 0.0,
            label: Some("luminance noise sigma".into()),
            evidence_region: None,
            confidence: Confidence::Low,
        };
    }
    let mut residuals = Vec::with_capacity(pw * ph);
    for y in 1..ph - 1 {
        for x in 1..pw - 1 {
            let v = preview[(0usize, y, x)] as f64;
            // 3x3 box median — cheap, sufficient for sigma-clipping.
            let mut window = [0.0f64; 9];
            for (i, dy) in [-1, 0, 1].iter().enumerate() {
                for (j, dx) in [-1, 0, 1].iter().enumerate() {
                    let yy = (y as i32 + dy) as usize;
                    let xx = (x as i32 + dx) as usize;
                    window[i * 3 + j] = preview[(0usize, yy, xx)] as f64;
                }
            }
            window.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
            let median = window[4];
            residuals.push((v - median).abs());
        }
    }
    if residuals.is_empty() {
        return MetricsSample {
            value: 0.0,
            label: Some("luminance noise sigma".into()),
            evidence_region: None,
            confidence: Confidence::Low,
        };
    }
    // Robust sigma via median absolute deviation (MAD):
    // sigma ≈ 1.4826 × MAD.
    let mut sorted = residuals.clone();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let mad = sorted[sorted.len() / 2];
    let sigma = 1.4826 * mad;
    let confidence = if residuals.len() >= 4096 {
        Confidence::High
    } else if residuals.len() >= 1024 {
        Confidence::Medium
    } else {
        Confidence::Low
    };
    MetricsSample {
        value: sigma,
        label: Some("luminance noise sigma".into()),
        evidence_region: Some([0, 0, w as u32, h as u32]),
        confidence,
    }
}

/// CR-06 §5.1 — Background gradient: slope of the per-tile
/// median over the image. Returns the slope magnitude in
/// pixel-units per 100 pixels (so a flat image returns ~0
/// and a typical gradient returns ~5–50).
///
/// We use the per-tile median to avoid stars skewing the
/// background estimate; tiles are 64×64 by default and the
/// result is the slope of a least-squares fit to the
/// tile-median field.
pub fn background_gradient(image: &F32Image) -> MetricsSample {
    let (w, h) = (image.width(), image.height());
    if w < 64 || h < 64 {
        return MetricsSample {
            value: 0.0,
            label: Some("background gradient magnitude (per 100 px)".into()),
            evidence_region: None,
            confidence: Confidence::Low,
        };
    }
    let tile = 64usize;
    let mut xs = Vec::new();
    let mut ys = Vec::new();
    let mut vs = Vec::new();
    let mut y0 = 0;
    while y0 < h {
        let mut x0 = 0;
        while x0 < w {
            let x1 = (x0 + tile).min(w);
            let y1 = (y0 + tile).min(h);
            // Sample a 4×4 grid of pixels per tile and take
            // the median — cheap, robust.
            let mut samples = Vec::with_capacity(16);
            for sy in (y0..y1).step_by((y1 - y0).max(1) / 4 + 1) {
                for sx in (x0..x1).step_by((x1 - x0).max(1) / 4 + 1) {
                    if sx < x1 && sy < y1 {
                        samples.push(image[(0, sy, sx)] as f64);
                    }
                }
            }
            if !samples.is_empty() {
                samples.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
                let med = samples[samples.len() / 2];
                xs.push((x0 as f64 + x1 as f64) / 2.0);
                ys.push((y0 as f64 + y1 as f64) / 2.0);
                vs.push(med);
            }
            x0 += tile;
        }
        y0 += tile;
    }
    if vs.len() < 2 {
        return MetricsSample {
            value: 0.0,
            label: Some("background gradient magnitude (per 100 px)".into()),
            evidence_region: None,
            confidence: Confidence::Low,
        };
    }
    // Fit plane z = a*x + b*y + c via least-squares and
    // return sqrt(a² + b²) as the gradient magnitude.
    let n = vs.len() as f64;
    let sx = xs.iter().sum::<f64>() / n;
    let sy = ys.iter().sum::<f64>() / n;
    let sz = vs.iter().sum::<f64>() / n;
    let sxx = xs.iter().map(|x| x * x).sum::<f64>() / n;
    let sxy = xs.iter().zip(ys.iter()).map(|(x, y)| x * y).sum::<f64>() / n;
    let sxz = xs.iter().zip(vs.iter()).map(|(x, v)| x * v).sum::<f64>() / n;
    let syy = ys.iter().map(|y| y * y).sum::<f64>() / n;
    let syz = ys.iter().zip(vs.iter()).map(|(y, v)| y * v).sum::<f64>() / n;
    let denom = (sxx - sx * sx) * (syy - sy * sy) - (sxy - sx * sy).powi(2);
    if denom.abs() < 1e-12 {
        return MetricsSample {
            value: 0.0,
            label: Some("background gradient magnitude (per 100 px)".into()),
            evidence_region: None,
            confidence: Confidence::Low,
        };
    }
    let a = ((syy - sy * sy) * (sxz - sx * sz) - (sxy - sx * sy) * (syz - sy * sz)) / denom;
    let b = ((sxx - sx * sx) * (syz - sy * sz) - (sxy - sx * sy) * (sxz - sx * sz)) / denom;
    let mag_per_px = (a * a + b * b).sqrt();
    let value = mag_per_px * 100.0;
    MetricsSample {
        value,
        label: Some("background gradient magnitude (per 100 px)".into()),
        evidence_region: Some([0, 0, w as u32, h as u32]),
        confidence: if vs.len() >= 16 {
            Confidence::High
        } else if vs.len() >= 4 {
            Confidence::Medium
        } else {
            Confidence::Low
        },
    }
}

/// CR-06 §5.1 — Clipping: fraction of pixels at or near the
/// top of the dynamic range. The image is assumed to be in
/// `[0, 1]` (the standard output of the CR-02 calibration
/// pipeline). Returns the fraction in `[0, 1]`.
pub fn highlight_clipping(image: &F32Image) -> MetricsSample {
    let total = (image.width() * image.height() * image.channels()) as f64;
    if total == 0.0 {
        return MetricsSample {
            value: 0.0,
            label: Some("highlight clipping fraction".into()),
            evidence_region: None,
            confidence: Confidence::Low,
        };
    }
    let mut clipped = 0usize;
    for v in image.iter() {
        if *v >= 0.999 {
            clipped += 1;
        }
    }
    let value = clipped as f64 / total;
    MetricsSample {
        value,
        label: Some("highlight clipping fraction".into()),
        evidence_region: Some([0, 0, image.width() as u32, image.height() as u32]),
        confidence: Confidence::Medium,
    }
}

/// CR-07 §8: Channel saturation percentage. Distinct from
/// `highlight_clipping`: that detector counts *channels* whose
/// value is at or near `1.0` (per-channel highlight clip).
/// This detector counts *pixels* whose R, G, AND B channels are
/// all at or near `1.0` (a fully-saturated pixel where color
/// information is lost across all three channels).
///
/// Astrophoto context: when bright stars or nebula cores saturate
/// the detector, all three channels read `1.0` and the
/// resulting pixel carries no color information: it just reads
/// as white. This is a different signal from "some channel is
/// clipped" (which can still preserve color via the unclipped
/// channels). A high fraction of fully-saturated pixels means
/// the highlights are losing color regardless of how the
/// per-channel highlights read.
///
/// Returns the fraction in `[0, 1]`. For monochrome / single-channel
/// images the value collapses to the same numerator/denominator
/// relationship as highlight_clipping (since "all channels"
/// is just "the one channel"); the detector gracefully degrades
/// to the per-pixel highlight-clipping fraction. Multi-channel
/// non-RGB images (e.g. narrowband Ha+OIII+SII+L) report the
/// fraction of *all* channels simultaneously saturated, which
/// for those workflows is a meaningful signal (all four bands
/// at `1.0`).
pub fn saturation_percentage(image: &F32Image) -> MetricsSample {
    let (w, h, c) = (image.width(), image.height(), image.channels());
    let total_pixels = (w * h) as f64;
    if total_pixels == 0.0 || c == 0 {
        return MetricsSample {
            value: 0.0,
            label: Some("saturation percentage".into()),
            evidence_region: None,
            confidence: Confidence::Low,
        };
    }
    // The F32Image layout is (channels, height, width) in
    // ndarray's C-order. Iteration order is `indexed_iter()`
    // over `Array3<f32>`, which visits ch=0's full block first
    // (all h*w pixels in row-major x-fastest order), then ch=1,
    // etc. Per-channel, every (y, x) is visited once.
    //
    // To detect "all C channels saturated at the same (y, x)",
    // we walk per-channel and record whether that channel's
    // sample at each pixel is saturated; the per-pixel AND
    // over all channels is the answer. The first channel
    // initialises the accumulator; subsequent channels keep
    // a pixel `true` only if every prior channel also had it
    // saturated.
    let mut saturated_count = vec![true; w * h];
    for ch_idx in 0..c {
        for y in 0..h {
            for x in 0..w {
                let v = image[(ch_idx, y, x)] as f64;
                let pixel_idx = y * w + x;
                if v < 0.999 {
                    saturated_count[pixel_idx] = false;
                }
            }
        }
    }
    let saturated = saturated_count.iter().filter(|&&s| s).count();
    let value = saturated as f64 / total_pixels;
    MetricsSample {
        value,
        label: Some("saturation percentage".into()),
        evidence_region: Some([0, 0, w as u32, h as u32]),
        confidence: Confidence::Medium,
    }
}

/// CR-06 §5.1 — Chromatic noise: difference between channels.
/// Returns the per-channel standard deviation of the
/// channel-mean ratios. A grayscale image returns 0.0; a
/// balanced colour image returns a small value; a colour
/// image with strong chromatic noise returns a larger
/// value.
pub fn chromatic_noise(image: &F32Image) -> MetricsSample {
    if image.channels() < 3 {
        return MetricsSample {
            value: 0.0,
            label: Some("chromatic noise (channel ratio std)".into()),
            evidence_region: None,
            confidence: Confidence::Low,
        };
    }
    let mut means = [0.0f64; 3];
    for c in 0..3 {
        let mut s = 0.0;
        let mut n = 0usize;
        for y in 0..image.height() {
            for x in 0..image.width() {
                s += image[(c, y, x)] as f64;
                n += 1;
            }
        }
        means[c] = if n > 0 { s / n as f64 } else { 0.0 };
    }
    let total = means.iter().sum::<f64>();
    if total < 1e-12 {
        return MetricsSample {
            value: 0.0,
            label: Some("chromatic noise (channel ratio std)".into()),
            evidence_region: None,
            confidence: Confidence::Low,
        };
    }
    let ratios: [f64; 3] = [means[0] / total, means[1] / total, means[2] / total];
    let m = ratios.iter().sum::<f64>() / 3.0;
    let var = ratios.iter().map(|r| (r - m).powi(2)).sum::<f64>() / 3.0;
    MetricsSample {
        value: var.sqrt(),
        label: Some("chromatic noise (channel ratio std)".into()),
        evidence_region: Some([0, 0, image.width() as u32, image.height() as u32]),
        confidence: Confidence::Medium,
    }
}

/// CR-06 §5.1 — Local contrast: mean absolute difference
/// between each pixel and its 3x3 neighbourhood mean.
/// Cheap heuristic; ML-driven metrics land in a later
/// phase if needed.
pub fn local_contrast(image: &F32Image) -> MetricsSample {
    let (w, h) = (image.width(), image.height());
    if w < 3 || h < 3 {
        return MetricsSample {
            value: 0.0,
            label: Some("local contrast (mean |Δ3x3|)".into()),
            evidence_region: None,
            confidence: Confidence::Low,
        };
    }
    let preview = image.downsample_box(0.25);
    let (pw, ph) = (preview.width(), preview.height());
    if pw < 3 || ph < 3 {
        return MetricsSample {
            value: 0.0,
            label: Some("local contrast (mean |Δ3x3|)".into()),
            evidence_region: None,
            confidence: Confidence::Low,
        };
    }
    let mut sum = 0.0f64;
    let mut n = 0usize;
    for y in 1..ph - 1 {
        for x in 1..pw - 1 {
            let v = preview[(0usize, y, x)] as f64;
            let mut acc = 0.0;
            for dy in [-1, 0, 1].iter() {
                for dx in [-1, 0, 1].iter() {
                    let yy = (y as i32 + dy) as usize;
                    let xx = (x as i32 + dx) as usize;
                    acc += preview[(0usize, yy, xx)] as f64;
                }
            }
            let mean = acc / 9.0;
            sum += (v - mean).abs();
            n += 1;
        }
    }
    let value = if n > 0 { sum / n as f64 } else { 0.0 };
    MetricsSample {
        value,
        label: Some("local contrast (mean |Δ3x3|)".into()),
        evidence_region: Some([0, 0, w as u32, h as u32]),
        confidence: if n >= 4096 {
            Confidence::High
        } else if n >= 256 {
            Confidence::Medium
        } else {
            Confidence::Low
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn noise_is_zero_for_uniform_image() {
        let img = F32Image::new(64, 64, 1);
        let sample = luminance_noise(&img);
        assert!(sample.value.abs() < 1e-6);
    }

    #[test]
    fn noise_is_nonzero_for_noisy_image() {
        // The detector downsamples to 0.25 then computes
        // median-based MAD. To exercise the detector the
        // test image must be small enough to skip the
        // downsampling path. `16×16` falls under the
        // `pw < 3 || ph < 3` short-circuit only at
        // smaller sizes, so we use a slightly larger image
        // and a high-amplitude noise pattern.
        let mut img = F32Image::new(64, 64, 1);
        for (i, v) in img.iter_mut().enumerate() {
            // Per-pixel hash with strong amplitude. The
            // detector's median-based MAD is sensitive to
            // per-pixel residuals even after a small
            // box average.
            let h = (i as u32).wrapping_mul(2654435761);
            *v = ((h % 1000) as f32) / 1000.0;
        }
        let sample = luminance_noise(&img);
        assert!(
            sample.value > 0.0,
            "expected nonzero noise sigma, got {}",
            sample.value
        );
    }

    #[test]
    fn background_gradient_detects_linear_ramp() {
        let mut img = F32Image::new(128, 128, 1);
        for y in 0..128 {
            for x in 0..128 {
                img[(0, y, x)] = (x as f32) / 256.0;
            }
        }
        let sample = background_gradient(&img);
        assert!(sample.value > 0.0);
    }

    #[test]
    fn background_gradient_is_near_zero_for_uniform_image() {
        let img = F32Image::new(128, 128, 1);
        let sample = background_gradient(&img);
        assert!(sample.value.abs() < 1e-6);
    }

    #[test]
    fn highlight_clipping_detects_clipped_image() {
        let mut img = F32Image::new(32, 32, 1);
        for v in img.iter_mut() {
            *v = 1.0;
        }
        let sample = highlight_clipping(&img);
        assert!((sample.value - 1.0).abs() < 1e-6);
    }

    #[test]
    fn highlight_clipping_is_zero_for_safe_image() {
        let mut img = F32Image::new(32, 32, 1);
        for v in img.iter_mut() {
            *v = 0.5;
        }
        let sample = highlight_clipping(&img);
        assert!(sample.value.abs() < 1e-6);
    }

    // CR-07 §8: saturation_percentage detector. Distinct
    // from highlight_clipping: counts pixels where ALL
    // channels are saturated, not just any single channel.

    /// 32x32 RGB image with every pixel fully saturated on
    /// all three channels: 100% saturation.
    fn fully_saturated_rgb(w: usize, h: usize) -> F32Image {
        let mut img = F32Image::new(w, h, 3);
        for v in img.iter_mut() {
            *v = 1.0;
        }
        img
    }

    /// 32x32 RGB image with no channel near saturation:
    /// 0% saturation.
    fn safe_rgb_image(w: usize, h: usize) -> F32Image {
        let mut img = F32Image::new(w, h, 3);
        for v in img.iter_mut() {
            *v = 0.5;
        }
        img
    }

    /// 32x32 RGB image where only the R channel is
    /// saturated (G and B at 0.5). Should report 0%
    /// saturation because not all channels are saturated
    /// on the same pixel.
    fn red_only_saturated_rgb(w: usize, h: usize) -> F32Image {
        let mut img = F32Image::new(w, h, 3);
        // The F32Image iterates (channels, height, width) in
        // C-order. For 3 channels at w*h pixels, the first
        // w*h values are R, the next are G, the next are B.
        let total = w * h;
        for (i, v) in img.iter_mut().enumerate() {
            if i < total {
                *v = 1.0; // R saturated
            } else {
                *v = 0.5; // G, B safe
            }
        }
        img
    }

    #[test]
    fn saturation_percentage_is_full_for_fully_saturated_rgb() {
        let img = fully_saturated_rgb(32, 32);
        let sample = saturation_percentage(&img);
        assert!(
            (sample.value - 1.0).abs() < 1e-6,
            "expected 100% saturation, got {}",
            sample.value
        );
    }

    #[test]
    fn saturation_percentage_is_zero_for_safe_rgb() {
        let img = safe_rgb_image(32, 32);
        let sample = saturation_percentage(&img);
        assert!(
            sample.value.abs() < 1e-6,
            "expected 0% saturation, got {}",
            sample.value
        );
    }

    #[test]
    fn saturation_percentage_is_zero_when_only_one_channel_is_saturated() {
        // R is fully saturated, G and B are at 0.5.
        // Per-channel highlight_clipping would report
        // (1/3) of channels clipped; saturation_percentage
        // should report 0% because no pixel has all
        // three channels at 1.0.
        let img = red_only_saturated_rgb(32, 32);
        let sample = saturation_percentage(&img);
        assert!(
            sample.value.abs() < 1e-6,
            "expected 0% saturation (red-only clip is not full saturation), got {}",
            sample.value
        );
    }

    #[test]
    fn saturation_percentage_distinguishes_from_highlight_clipping() {
        // Build a 32x32 RGB image with only R clipped.
        // highlight_clipping sees 1/3 of channels clipped
        // (value ≈ 0.333). saturation_percentage sees 0
        // pixels with all channels saturated (value = 0).
        let img = red_only_saturated_rgb(32, 32);
        let clip = highlight_clipping(&img);
        let sat = saturation_percentage(&img);
        assert!(
            clip.value > 0.3 && clip.value < 0.4,
            "highlight_clipping should report ~1/3, got {}",
            clip.value
        );
        assert!(
            sat.value.abs() < 1e-6,
            "saturation_percentage should be 0 (red-only clip), got {}",
            sat.value
        );
    }

    #[test]
    fn saturation_percentage_handles_mixed_pixels() {
        // 16x16 RGB image: top half (8 rows × 16 px) fully
        // saturated on all channels, bottom half safe.
        // Expected: 50% of pixels saturated.
        let mut img = F32Image::new(16, 16, 3);
        for ((_ch, y, _x), v) in img.indexed_iter_mut() {
            if y < 8 {
                *v = 1.0;
            } else {
                *v = 0.5;
            }
        }
        let sample = saturation_percentage(&img);
        assert!(
            (sample.value - 0.5).abs() < 1e-6,
            "expected 50% saturation, got {}",
            sample.value
        );
    }

    #[test]
    fn chromatic_noise_is_zero_for_balanced_image() {
        let img = F32Image::new(32, 32, 3);
        let sample = chromatic_noise(&img);
        assert!(sample.value.abs() < 1e-6);
    }

    #[test]
    fn local_contrast_is_zero_for_uniform_image() {
        let img = F32Image::new(64, 64, 1);
        let sample = local_contrast(&img);
        assert!(sample.value.abs() < 1e-6);
    }

    #[test]
    fn local_contrast_is_nonzero_for_step_image() {
        let mut img = F32Image::new(64, 64, 1);
        for y in 0..64 {
            for x in 0..64 {
                img[(0, y, x)] = if x < 32 { 0.0 } else { 1.0 };
            }
        }
        let sample = local_contrast(&img);
        assert!(sample.value > 0.0);
    }
}
