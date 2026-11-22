//! Tone mapping — display stretch selection over the core math.

use astroforge_core::image::F32Image;
use astroforge_core::stretching::{self, AutoStretchParams};

/// How linear-light pixels are mapped into display range before the
/// u8 conversion in `color::to_color_image`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ToneMap {
    /// Raw values, clamped to [0, 1]. Right for previews that were
    /// already stretched when written.
    None,
    /// Screen-stretch (arcsinh midtone) computed from the image.
    /// The web app's MTF shader equivalent.
    AutoStretch,
    /// Re-apply a previously computed stretch (keeps the tone stable
    /// across repaints / recomposites).
    Params(AutoStretchParams),
}

impl ToneMap {
    /// Apply to a cloned image; returns (display image, params that
    /// were used — so callers can persist/restore the exact stretch).
    pub fn apply(&self, image: &F32Image) -> (F32Image, AutoStretchParams) {
        match self {
            ToneMap::None => (
                image.clone(),
                AutoStretchParams {
                    min: 0.0,
                    max: 1.0,
                    midtones: 0.0,
                },
            ),
            ToneMap::AutoStretch => stretching::auto_stretch_with_params(image),
            // Forward application of stored params: the exact math of
            // `auto_stretch_with_params` driven by the remembered
            // values (core exposes arcsinh_stretch; the normalize +
            // stretch loop mirrors it line-for-line).
            ToneMap::Params(params) => {
                let mut out = image.clone();
                let range = (params.max - params.min).max(1e-10);
                for val in out.iter_mut() {
                    let normalized = f64::from((*val - params.min) / range);
                    *val = stretching::arcsinh_stretch(normalized, params.midtones) as f32;
                }
                (out, *params)
            }
        }
    }
}
