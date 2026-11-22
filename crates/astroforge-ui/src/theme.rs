//! egui theme mapping — the ONLY place design tokens meet egui's
//! `Style`/`Visuals`. No color or size literals here; everything is
//! derived from `tokens.rs` (which is ported from `docs/DESIGN.md`).

use egui::{Stroke, TextStyle};

use crate::tokens as t;

/// Apply the theme: visuals + text styles + spacing, all from
/// tokens. Call once from `AstroForgeApp::new`.
pub fn install(ctx: &egui::Context) {
    use t::{accent, border, surface, text};

    ctx.set_visuals(egui::Visuals::dark());
    ctx.all_styles_mut(|style| {
        let v = &mut style.visuals;

        // Surfaces: tonal elevation scale.
        v.extreme_bg_color = surface::CONTAINER_LOWEST;
        v.faint_bg_color = surface::CONTAINER_LOW;
        v.window_fill = surface::CONTAINER;
        v.window_stroke = Stroke::new(1.0, border::HAIRLINE);
        v.selection.bg_fill = t::elevation::FOCUS_RING.color;
        v.selection.stroke = Stroke::new(1.0, accent::PRIMARY);
        v.hyperlink_color = accent::TERTIARY;
        v.warn_fg_color = t::Status::Warning.fg();
        v.error_fg_color = t::Status::Error.fg();
        v.override_text_color = Some(text::PRIMARY);

        // Widget states walk the surface scale (idle -> container,
        // hover -> container-high, active -> container-highest).
        let w = &mut v.widgets;
        w.noninteractive.bg_fill = surface::BASE;
        w.noninteractive.weak_bg_fill = surface::BASE;
        w.noninteractive.bg_stroke = Stroke::new(1.0, border::HAIRLINE);
        w.noninteractive.fg_stroke = Stroke::new(1.0, text::PRIMARY);
        w.noninteractive.corner_radius = t::radius::BASE;

        w.inactive.weak_bg_fill = surface::CONTAINER;
        w.inactive.bg_stroke = Stroke::new(1.0, border::HAIRLINE);
        w.inactive.fg_stroke = Stroke::new(1.0, text::PRIMARY);
        w.inactive.corner_radius = t::radius::BASE;

        w.hovered.weak_bg_fill = surface::CONTAINER_HIGH;
        w.hovered.bg_stroke = Stroke::new(1.0, border::STRONG);
        w.hovered.fg_stroke = Stroke::new(1.0, text::PRIMARY);
        w.hovered.corner_radius = t::radius::BASE;

        w.active.weak_bg_fill = surface::CONTAINER_HIGHEST;
        w.active.bg_stroke = t::elevation::FOCUS_RING;
        w.active.fg_stroke = Stroke::new(1.0, accent::PRIMARY);
        w.active.corner_radius = t::radius::BASE;

        w.open.weak_bg_fill = surface::CONTAINER_HIGHEST;
        w.open.bg_stroke = Stroke::new(1.0, border::STRONG);
        w.open.fg_stroke = Stroke::new(1.0, text::PRIMARY);
        w.open.corner_radius = t::radius::BASE;

        // Typography from the type-scale tokens.
        style.text_styles = t::egui_text_styles()
            .into_iter()
            .collect::<std::collections::BTreeMap<TextStyle, egui::FontId>>();

        // Spacing: 4px grid; sharp 150ms transitions (DESIGN.md
        // "Interactions").
        style.spacing.item_spacing = egui::vec2(t::spacing::SM, t::spacing::SM);
        style.spacing.button_padding = egui::vec2(
            t::spacing::SM + t::spacing::UNIT,
            t::spacing::XS + t::spacing::UNIT,
        );
        style.spacing.menu_margin = egui::Margin::same(t::spacing::SM as i8);
        style.animation_time = t::egui_animation_time();
    });
}
