//! Bundled fonts + egui font-family wiring.
//!
//! The web app loads Geist / Inter / Space Grotesk / JetBrains Mono
//! from a CDN (`index.html`). A native app bundles its fonts: Inter
//! (body + heading weights) and JetBrains Mono (data / code). Space
//! Grotesk is skipped for now — headings use Inter SemiBold until a
//! later slice re-evaluates (only static TTF instances are bundled;
//! Material Symbols ligature icons do not work in egui and will be
//! replaced by SVG icons in P2).

use egui::epaint::text::FontDefinitions;
use egui::{FontData, FontFamily};

const INTER_REGULAR: &[u8] = include_bytes!("../assets/fonts/Inter-Regular.ttf");
const INTER_MEDIUM: &[u8] = include_bytes!("../assets/fonts/Inter-Medium.ttf");
const INTER_SEMIBOLD: &[u8] = include_bytes!("../assets/fonts/Inter-SemiBold.ttf");
const INTER_BOLD: &[u8] = include_bytes!("../assets/fonts/Inter-Bold.ttf");
const JBMONO_REGULAR: &[u8] = include_bytes!("../assets/fonts/JetBrainsMono-Regular.ttf");
const JBMONO_MEDIUM: &[u8] = include_bytes!("../assets/fonts/JetBrainsMono-Medium.ttf");

/// Install the bundled fonts. Inter replaces the egui default
/// (Ubuntu-Light) as the primary proportional face; the egui emoji
/// fonts stay as fallbacks so special glyphs keep rendering.
pub fn install(ctx: &egui::Context) {
    let mut fonts = FontDefinitions::default();
    fonts.font_data.insert(
        "inter_regular".into(),
        FontData::from_static(INTER_REGULAR).into(),
    );
    fonts.font_data.insert(
        "inter_medium".into(),
        FontData::from_static(INTER_MEDIUM).into(),
    );
    fonts.font_data.insert(
        "inter_semibold".into(),
        FontData::from_static(INTER_SEMIBOLD).into(),
    );
    fonts.font_data.insert(
        "inter_bold".into(),
        FontData::from_static(INTER_BOLD).into(),
    );
    fonts.font_data.insert(
        "jbmono_regular".into(),
        FontData::from_static(JBMONO_REGULAR).into(),
    );
    fonts.font_data.insert(
        "jbmono_medium".into(),
        FontData::from_static(JBMONO_MEDIUM).into(),
    );

    // Primary faces (defaults stay as fallbacks for emoji / symbols).
    fonts
        .families
        .get_mut(&FontFamily::Proportional)
        .expect("proportional family exists")
        .insert(0, "inter_regular".into());
    fonts
        .families
        .get_mut(&FontFamily::Monospace)
        .expect("monospace family exists")
        .insert(0, "jbmono_regular".into());

    // Weighted families for text styles (see theme::install).
    fonts.families.insert(
        FontFamily::Name("heading".into()),
        vec!["inter_semibold".into(), "inter_regular".into()],
    );
    fonts.families.insert(
        FontFamily::Name("medium".into()),
        vec!["inter_medium".into(), "inter_regular".into()],
    );
    fonts.families.insert(
        FontFamily::Name("mono_medium".into()),
        vec!["jbmono_medium".into(), "jbmono_regular".into()],
    );

    ctx.set_fonts(fonts);
}
