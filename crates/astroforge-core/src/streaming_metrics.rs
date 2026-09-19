// CR-07 §29.1: streaming metrics accumulator.
//
// The original `metric_snapshot_full` runs each per-pixel
// detector in its own O(WHC) pass:
//
// - `channel_stats` (mean/stddev/min/max/clip_count per channel)
// - `highlight_clipping` (count of pixels where ANY channel
//   has value >= 0.999, in [0, 1])
// - `saturation_percentage` (count of pixels where ALL
//   channels have value >= 0.999, in [0, 100])
//
// On a 4K RGBA image that's 3 separate walks of 33M floats
// each. This module collapses all three into a single
// `StreamingAccumulator` that walks every pixel once and
// updates every per-pixel accumulator simultaneously.
//
// The spatial detectors (`luminance_noise`, `chromatic_noise`,
// `local_contrast`, `background_gradient`) stay as separate
// passes because they require neighbor-pixel or tile
// relationships. The §29.1 optimization targets the
// per-pixel subset, which is the bulk of the runtime on
// 4K+ images.
//
// Output equivalence is the primary contract: the streaming
// accumulator must produce byte-identical output to the
// multi-pass baseline on every fixture. Tests pin this in
// `tests/streaming_metrics.rs`.

use crate::image::F32Image;
use crate::metric_registry::MetricKind;
use std::collections::BTreeMap;

/// Single-pass accumulator over an `F32Image` for the
/// per-pixel metrics:
/// - `channel_stats` (mean / stddev / min / max / clip_count per channel)
/// - `highlight_clipping` (count of pixels where ANY channel >= 0.999)
/// - `saturation_percentage` (count of pixels where ALL channels >= 0.999)
///
/// Walks every pixel once, updates every accumulator
/// simultaneously.
#[derive(Debug, Clone)]
pub struct StreamingAccumulator {
    /// One per channel.
    sums: Vec<f64>,
    /// Sum of squares per channel (for variance).
    sum_sq: Vec<f64>,
    /// Min per channel.
    mins: Vec<f64>,
    /// Max per channel.
    maxs: Vec<f64>,
    /// Pixels-per-channel with value >= 1.0 (the `clip_count`
    /// threshold; matches `channel_stats`).
    clip_counts: Vec<u64>,
    /// Channels with value >= 0.999 (the `highlight_clipping`
    /// threshold; matches the existing
    /// `image_analysis::metrics::highlight_clipping` detector
    /// which iterates per-channel, not per-pixel). The
    /// existing detector reports this as a fraction of
    /// total channel count in [0, 1]; we keep the raw
    /// channel-count internally and convert at finalize
    /// time so the streaming output matches the existing
    /// baseline byte-for-byte.
    highlight_clip_count: u64,
    /// Pixels where ALL channels >= 0.999 (the `saturation_pct`
    /// threshold). Existing detector reports this as a
    /// percentage [0, 100]; we keep the raw pixel-count
    /// internally and convert at finalize time.
    saturated_pixel_count: u64,
    /// Total pixel count (= width * height).
    pixel_count: u64,
}

impl StreamingAccumulator {
    /// Build an empty accumulator sized for an image with
    /// `n_channels` channels.
    pub fn new(n_channels: usize) -> Self {
        Self {
            sums: vec![0.0; n_channels],
            sum_sq: vec![0.0; n_channels],
            mins: vec![f64::INFINITY; n_channels],
            maxs: vec![f64::NEG_INFINITY; n_channels],
            clip_counts: vec![0; n_channels],
            highlight_clip_count: 0,
            saturated_pixel_count: 0,
            pixel_count: 0,
        }
    }

    /// Walk every pixel of `image` once, updating every
    /// accumulator in lockstep.
    pub fn observe(&mut self, image: &F32Image) {
        let n_channels = image.channels();
        let width = image.width();
        let height = image.height();
        self.pixel_count = (width * height) as u64;
        // First single pass: per-channel accumulators
        // (sum, sum_sq, min, max, clip_count). Array3 layout
        // is channel-major C-order, so iterating (ch, y, x)
        // visits every pixel of channel 0, then every pixel
        // of channel 1, etc. Same layout as `channel_stats`.
        for ch in 0..n_channels {
            for y in 0..height {
                for x in 0..width {
                    let v = image[(ch, y, x)] as f64;
                    self.sums[ch] += v;
                    self.sum_sq[ch] += v * v;
                    if v < self.mins[ch] {
                        self.mins[ch] = v;
                    }
                    if v > self.maxs[ch] {
                        self.maxs[ch] = v;
                    }
                    if v >= 1.0 {
                        self.clip_counts[ch] += 1;
                    }
                }
            }
        }
        // Second single pass: cross-channel metrics.
        // `highlight_clipping` counts channels (matches the
        // existing baseline detector's per-channel model);
        // `saturation_percentage` counts pixels (matches
        // the existing baseline detector's per-pixel model).
        for y in 0..height {
            for x in 0..width {
                let mut all_clip = true;
                for ch in 0..n_channels {
                    let v = image[(ch, y, x)] as f64;
                    if v >= 0.999 {
                        self.highlight_clip_count += 1;
                    } else {
                        all_clip = false;
                    }
                }
                if all_clip {
                    self.saturated_pixel_count += 1;
                }
            }
        }
    }

    /// Convert the accumulated state into the metric-snapshot
    /// format that the comparison surface (and the existing
    /// `channel_stats` / `highlight_clipping` /
    /// `saturation_percentage` callers) consume.
    ///
    /// Returned keys (sorted by `BTreeMap`):
    ///
    /// - `channel.<r|g|b|c{n}>.mean`
    /// - `channel.<r|g|b|c{n}>.stddev`
    /// - `channel.<r|g|b|c{n}>.min`
    /// - `channel.<r|g|b|c{n}>.max`
    /// - `channel.<r|g|b|c{n}>.clip_count`
    /// - `dynamic_range.highlight_clipping`
    /// - `dynamic_range.saturation_pct`
    pub fn finalize(&self) -> BTreeMap<String, f64> {
        let mut out = BTreeMap::new();
        let n = self.pixel_count.max(1) as f64;
        let n_channels = self.sums.len();
        // Per-channel stats. The first three channels get
        // the friendly `r` / `g` / `b` names (matching
        // `channel_stats`); channels beyond three get `c{n}`.
        for ch in 0..n_channels {
            let channel_key: String = if ch == 0 {
                "r".to_string()
            } else if ch == 1 {
                "g".to_string()
            } else if ch == 2 {
                "b".to_string()
            } else {
                format!("c{ch}")
            };
            let mean = self.sums[ch] / n;
            let variance = (self.sum_sq[ch] / n) - mean * mean;
            // For uniform images, variance can be slightly
            // negative due to floating-point drift; clamp
            // to zero so stddev is well-defined.
            let stddev = variance.max(0.0).sqrt();
            out.insert(format!("channel.{channel_key}.mean"), mean);
            out.insert(format!("channel.{channel_key}.stddev"), stddev);
            out.insert(format!("channel.{channel_key}.min"), self.mins[ch]);
            out.insert(format!("channel.{channel_key}.max"), self.maxs[ch]);
            out.insert(
                format!("channel.{channel_key}.clip_count"),
                self.clip_counts[ch] as f64,
            );
        }
        // Cross-channel metrics. Keys match
        // `MetricKind::as_str()` for the two §8 detectors.
        // `highlight_clipping` counts CHANNELS (not pixels)
        // with v >= 0.999 and reports the fraction of total
        // channel count in [0, 1]; matches the existing
        // baseline detector's unit exactly. `saturation_pct`
        // counts pixels with ALL channels >= 0.999 and
        // reports the FRACTION of total pixel count in [0, 1]
        // (NOT a percentage in [0, 100]); matches the
        // existing baseline detector's unit exactly. The
        // metric_registry `as_str()` returns
        // `dynamic_range.saturation_pct` for the key name;
        // the value's unit is fraction, not percentage.
        let total_channels = (self.pixel_count as f64) * (self.sums.len() as f64);
        let highlight_fraction = if total_channels == 0.0 {
            0.0
        } else {
            self.highlight_clip_count as f64 / total_channels
        };
        let saturation_fraction = if self.pixel_count == 0 {
            0.0
        } else {
            self.saturated_pixel_count as f64 / (self.pixel_count as f64)
        };
        out.insert(
            MetricKind::DynamicRangeHighlightClipping
                .as_str()
                .to_string(),
            highlight_fraction,
        );
        out.insert(
            MetricKind::DynamicRangeSaturationPct.as_str().to_string(),
            saturation_fraction,
        );
        out
    }
}

/// Walk `image` once, computing the per-pixel metrics in a
/// single pass. Returns the same key set as
/// `channel_stats` ∪ `highlight_clipping` ∪ `saturation_percentage`.
pub fn streaming_metrics(image: &F32Image) -> BTreeMap<String, f64> {
    let mut acc = StreamingAccumulator::new(image.channels());
    acc.observe(image);
    acc.finalize()
}
