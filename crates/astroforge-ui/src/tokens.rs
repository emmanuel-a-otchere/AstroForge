//! AstroForge design tokens — the single source of truth for UI look
//! and feel in the native app.
//!
//! Ported verbatim from `docs/DESIGN.md` ("AstroForge Precision") and
//! the `:root` block in `src/app.css`. Screens and components must
//! consume THESE values, never raw hex / px literals; `theme.rs`
//! maps them onto egui's `Style`/`Visuals` once at startup, and
//! `semantics.rs` builds reusable components on top of them.
//!
//! Brand: "Scientific Modernism" — professional observatory
//! workstation, Deep Space Dark Mode shifted toward spectral reds to
//! preserve night vision. Hairline 1px borders, tonal elevation over
//! shadows, strict 4px grid, sharp 150ms transitions.

use egui::{Stroke, TextStyle};

/// Hex helper so token definitions read like the DESIGN.md source.
const fn hex(rgb: u32) -> egui::Color32 {
    egui::Color32::from_rgb(
        (rgb >> 16) as u8,
        ((rgb >> 8) & 0xff) as u8,
        (rgb & 0xff) as u8,
    )
}

/// Alpha-composited tint of a base color (unmultiplied RGBA), used
/// for status pill backgrounds and borders (mirrors the CSS
/// `--success-border: #b9f0c566` pattern). Const because token
/// `Stroke`s are defined as `const` items.
const fn tint(rgb: u32, alpha: u8) -> egui::Color32 {
    egui::Color32::from_rgba_unmultiplied_const(
        (rgb >> 16) as u8,
        ((rgb >> 8) & 0xff) as u8,
        (rgb & 0xff) as u8,
        alpha,
    )
}

// ─── Color: surfaces ──────────────────────────────────────────────────────

pub mod surface {
    //! Technical Grey surface scale (tonal elevation, no shadows).
    use egui::Color32;

    use super::hex;

    pub const BASE: Color32 = hex(0x121414);
    pub const DIM: Color32 = hex(0x121414);
    pub const BRIGHT: Color32 = hex(0x37393a);
    pub const CONTAINER_LOWEST: Color32 = hex(0x0c0f0f);
    pub const CONTAINER_LOW: Color32 = hex(0x1a1c1c);
    pub const CONTAINER: Color32 = hex(0x1e2020);
    pub const CONTAINER_HIGH: Color32 = hex(0x282a2b);
    pub const CONTAINER_HIGHEST: Color32 = hex(0x333535);
    pub const VARIANT: Color32 = hex(0x333535);
    pub const INVERSE: Color32 = hex(0xe2e2e2);
    pub const INVERSE_ON: Color32 = hex(0x2f3131);
}

// ─── Color: text ──────────────────────────────────────────────────────────

pub mod text {
    use egui::Color32;

    use super::hex;

    pub const PRIMARY: Color32 = hex(0xe2e2e2);
    pub const SECONDARY: Color32 = hex(0xdfbfba);
    pub const ON_ACCENT: Color32 = hex(0xffffff);
    pub const INVERSE: Color32 = hex(0x2f3131);
}

// ─── Color: borders ───────────────────────────────────────────────────────

pub mod border {
    use egui::Color32;

    use super::hex;

    pub const STRONG: Color32 = hex(0xa78a86);
    /// Default hairline: 1px outlines delineate workspace modules.
    pub const HAIRLINE: Color32 = hex(0x58413e);
}

// ─── Color: brand accents ─────────────────────────────────────────────────

pub mod accent {
    use egui::Color32;

    use super::hex;

    /// Solar Flare Red — brand moments, primary buttons, active nodes.
    pub const PRIMARY: Color32 = hex(0xffb4a8);
    pub const PRIMARY_BRIGHT: Color32 = hex(0xcb4e3d);
    pub const PRIMARY_CONTAINER: Color32 = hex(0xec6653);
    pub const ON_PRIMARY: Color32 = hex(0x680301);

    /// Dust Muted Rose — the functional workhorse: interactive data
    /// points, active sliders, precise metrics.
    pub const SECONDARY_BRIGHT: Color32 = hex(0xa8655b);

    /// Deep Orbit Blue — subtle tertiary differentiator for grouping.
    pub const TERTIARY: Color32 = hex(0xbcc3ff);
    pub const TERTIARY_BRIGHT: Color32 = hex(0x7787f9);
    pub const TERTIARY_DEEP: Color32 = hex(0x001891);
}

// ─── Typography ───────────────────────────────────────────────────────────

pub mod type_scale {
    //! Sizes/families from the DESIGN.md `typography:` block.
    //!
    //! Font roles: Display/Directional = "heading" family (Inter
    //! SemiBold until Geist/Space Grotesk are bundled; see
    //! `fonts.rs`), Body = Inter, Technical/Data = monospace
    //! (JetBrains Mono, standing in for Space Grotesk). Swapping a
    //! role is a one-line change here — screens never pick families.

    use std::sync::LazyLock;

    use egui::{FontFamily, FontId};

    /// A resolved text style token.
    #[derive(Clone, Debug, PartialEq)]
    pub struct TextStyleToken {
        pub size: f32,
        pub family: FontFamily,
    }

    impl TextStyleToken {
        pub const fn new(size: f32, family: FontFamily) -> Self {
            Self { size, family }
        }

        pub fn font_id(&self) -> FontId {
            FontId::new(self.size, self.family.clone())
        }
    }

    fn heading_family() -> FontFamily {
        FontFamily::Name("heading".into())
    }

    fn mono_medium_family() -> FontFamily {
        FontFamily::Name("mono_medium".into())
    }

    /// 48/700 — hero moments only (welcome screen).
    pub static DISPLAY_XL: LazyLock<TextStyleToken> =
        LazyLock::new(|| TextStyleToken::new(48.0, heading_family()));
    /// 32/600 — page titles.
    pub static HEADLINE_LG: LazyLock<TextStyleToken> =
        LazyLock::new(|| TextStyleToken::new(32.0, heading_family()));
    /// 24/600 — panel titles.
    pub static HEADLINE: LazyLock<TextStyleToken> =
        LazyLock::new(|| TextStyleToken::new(24.0, heading_family()));
    /// 15/400 — all body copy and instructions.
    pub static BODY: LazyLock<TextStyleToken> =
        LazyLock::new(|| TextStyleToken::new(15.0, FontFamily::Proportional));
    /// 14/500 — variable data: FITS headers, coordinates, metrics.
    pub static DATA: LazyLock<TextStyleToken> =
        LazyLock::new(|| TextStyleToken::new(14.0, FontFamily::Monospace));
    /// 12/400 — metadata lines.
    pub static METADATA: LazyLock<TextStyleToken> =
        LazyLock::new(|| TextStyleToken::new(12.0, FontFamily::Proportional));
    /// 11/700 — section card headers (uppercased at the call site).
    pub static LABEL_CAPS: LazyLock<TextStyleToken> =
        LazyLock::new(|| TextStyleToken::new(11.0, mono_medium_family()));
    /// Button text inherits BODY size, medium weight family.
    pub static BUTTON: LazyLock<TextStyleToken> =
        LazyLock::new(|| TextStyleToken::new(15.0, FontFamily::Name("medium".into())));
}

// ─── Spacing (4px baseline grid) ──────────────────────────────────────────

pub mod spacing {
    pub const UNIT: f32 = 4.0;
    pub const XS: f32 = 4.0;
    pub const SM: f32 = 8.0;
    pub const MD: f32 = 16.0;
    pub const LG: f32 = 24.0;
    pub const XL: f32 = 40.0;
    pub const PANEL_GUTTER: f32 = 1.0;
    pub const CONTAINER_SAFE: f32 = 24.0;
}

// ─── Shape ────────────────────────────────────────────────────────────────

pub mod radius {
    //! "Soft but disciplined": 4px for controls, 8px for cards, sharp
    //! for technical graphs, pill for status chips / pipeline nodes.
    use egui::CornerRadius;

    pub const SHARP: CornerRadius = CornerRadius::same(0);
    pub const SM: CornerRadius = CornerRadius::same(2);
    pub const BASE: CornerRadius = CornerRadius::same(4);
    pub const MD: CornerRadius = CornerRadius::same(6);
    pub const LG: CornerRadius = CornerRadius::same(8);
    pub const XL: CornerRadius = CornerRadius::same(12);
    pub const PILL: CornerRadius = CornerRadius::same(255);
}

// ─── Elevation & focus ────────────────────────────────────────────────────

pub mod elevation {
    //! Depth = tonal layering + hairline borders, NOT drop shadows.
    use egui::{Color32, Stroke};

    use super::{border, surface, tint};

    /// Level 1 (panels/cards): surface tier + 1px hairline border.
    pub const LEVEL1_FILL: Color32 = surface::CONTAINER_LOW;
    pub const LEVEL1_BORDER: Stroke = Stroke {
        width: 1.0,
        color: border::HAIRLINE,
    };

    /// Level 2 (overlays/modals): semi-transparent surface (85%).
    pub const LEVEL2_FILL: Color32 = tint(0x1e2020, 217);

    /// Active focus: Solar Flare Red 1px outer ring (the "glow").
    pub const FOCUS_RING: Stroke = Stroke {
        width: 1.0,
        color: tint(0xcb4e3d, 130),
    };

    /// Translucent scrims for dialogs.
    pub const SCRIM: Color32 = tint(0x000000, 153); // 60%
    pub const GLASS: Color32 = tint(0x121414, 153); // 60%
}

// ─── Motion ───────────────────────────────────────────────────────────────

pub mod motion {
    //! "Sharp transitions (150ms) for data updates."
    use std::time::Duration;

    pub const FAST: Duration = Duration::from_millis(100);
    pub const BASE: Duration = Duration::from_millis(150);
    pub const SLOW: Duration = Duration::from_millis(300);
}

// ─── Status semantics ─────────────────────────────────────────────────────

/// Semantic status vocabulary. Components consume this enum, never
/// raw colors; the palette lives here so every surface stays
/// consistent (DESIGN.md "Status Tints").
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Status {
    /// Idle / queued / structural states.
    Neutral,
    /// Review / informational moments (Deep Orbit Blue family).
    Info,
    /// Completed successfully (Emerald).
    Success,
    /// Work in flight (Dust Muted Rose).
    Processing,
    /// Paused / attention without failure (Amber).
    Warning,
    /// Failure (Crimson family).
    Error,
}

impl Status {
    /// Foreground (the LED / label color).
    pub fn fg(self) -> egui::Color32 {
        match self {
            Status::Neutral => surface::BRIGHT,
            Status::Info => accent::TERTIARY,
            Status::Success => hex(0x10b981),
            Status::Processing => accent::SECONDARY_BRIGHT,
            Status::Warning => hex(0xf59e0b),
            Status::Error => hex(0xffb4ab),
        }
    }

    /// Translucent pill fill (~10% of the fg hue).
    pub fn bg(self) -> egui::Color32 {
        match self {
            Status::Neutral => tint(0x37393a, 36),
            Status::Info => tint(0xbcc3ff, 26),
            Status::Success => tint(0x10b981, 26),
            Status::Processing => tint(0xa8655b, 36),
            Status::Warning => tint(0xf59e0b, 20),
            Status::Error => tint(0xffb4ab, 26),
        }
    }

    /// Hairline pill border (~40% alpha).
    pub fn border(self) -> Stroke {
        match self {
            Status::Neutral => Stroke::new(1.0, tint(0x37393a, 102)),
            Status::Info => Stroke::new(1.0, tint(0xbcc3ff, 102)),
            Status::Success => Stroke::new(1.0, tint(0x10b981, 102)),
            Status::Processing => Stroke::new(1.0, tint(0xa8655b, 102)),
            Status::Warning => Stroke::new(1.0, tint(0xf59e0b, 87)),
            Status::Error => Stroke::new(1.0, tint(0xffb4ab, 102)),
        }
    }
}

// ─── Theme application helpers ────────────────────────────────────────────

/// egui TextStyle map entries derived from the type scale.
pub fn egui_text_styles() -> [(TextStyle, egui::FontId); 5] {
    [
        (TextStyle::Heading, type_scale::HEADLINE.font_id()),
        (TextStyle::Body, type_scale::BODY.font_id()),
        (TextStyle::Button, type_scale::BUTTON.font_id()),
        (TextStyle::Small, type_scale::METADATA.font_id()),
        (TextStyle::Monospace, type_scale::DATA.font_id()),
    ]
}

/// Global egui animation duration from the motion tokens.
pub fn egui_animation_time() -> f32 {
    motion::BASE.as_secs_f32()
}
