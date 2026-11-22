//! Semantic UI components — the vocabulary screens are built from.
//!
//! Every component here consumes `tokens` only, so look-and-feel is
//! consistent by construction. The layer also owns UI *semantics*:
//! accessible roles/names, tooltips, status vocabulary, and context
//! patterns (empty states, section cards, key/value metadata).
//!
//! Rule of thumb for new screens: if a pattern appears twice, it
//! moves here.

use egui::{Color32, CornerRadius, Frame, Margin, Response, RichText, Ui};

use crate::tokens as t;

// ─── Typography ───────────────────────────────────────────────────────────

/// Page title (HEADLINE_LG token).
pub fn page_title(ui: &mut Ui, text: &str) {
    ui.label(
        RichText::new(text)
            .size(t::type_scale::HEADLINE_LG.size)
            .family(t::type_scale::HEADLINE_LG.family.clone())
            .color(t::text::PRIMARY),
    );
}

/// Section title inside content (HEADLINE token).
pub fn heading(ui: &mut Ui, text: &str) {
    ui.label(
        RichText::new(text)
            .size(t::type_scale::HEADLINE.size)
            .family(t::type_scale::HEADLINE.family.clone())
            .color(t::text::PRIMARY),
    );
}

/// Uppercase micro-label for card headers / column headers
/// (LABEL_CAPS token, muted color).
pub fn label_caps(ui: &mut Ui, text: &str) {
    ui.label(
        RichText::new(text.to_uppercase())
            .size(t::type_scale::LABEL_CAPS.size)
            .family(t::type_scale::LABEL_CAPS.family.clone())
            .color(t::border::STRONG),
    );
}

/// Body copy in the muted "on-surface-variant" tone.
pub fn body_muted(ui: &mut Ui, text: &str) {
    ui.label(
        RichText::new(text)
            .size(t::type_scale::BODY.size)
            .color(t::text::SECONDARY),
    );
}

/// Technical/data text (DATA token, mono family).
pub fn data_text(ui: &mut Ui, text: &str) {
    ui.label(
        RichText::new(text)
            .size(t::type_scale::DATA.size)
            .family(t::type_scale::DATA.family.clone())
            .color(t::text::PRIMARY),
    );
}

// ─── Containers ───────────────────────────────────────────────────────────

/// Level-1 card (DESIGN.md "Cards"): tonal fill + hairline border +
/// label-caps header over a 1px divider. The body closure draws the
/// card contents.
pub fn section<R>(ui: &mut Ui, title: &str, add_contents: impl FnOnce(&mut Ui) -> R) {
    Frame::default()
        .fill(t::elevation::LEVEL1_FILL)
        .stroke(t::elevation::LEVEL1_BORDER)
        .corner_radius(t::radius::LG)
        .inner_margin(Margin::same(t::spacing::MD as i8))
        .show(ui, |ui| {
            label_caps(ui, title);
            ui.add_space(t::spacing::XS);
            ui.separator();
            ui.add_space(t::spacing::XS);
            add_contents(ui);
        });
}

/// Aligned key/value metadata rows: key in muted metadata tone,
/// value in DATA (mono). Use for provenance/settings-style blocks.
pub fn kv_table(ui: &mut Ui, rows: &[(&str, &str)]) {
    egui::Grid::new("kv_table")
        .num_columns(2)
        .spacing([t::spacing::MD, t::spacing::SM])
        .show(ui, |ui| {
            for (key, value) in rows {
                ui.label(
                    RichText::new(*key)
                        .size(t::type_scale::METADATA.size)
                        .color(t::text::SECONDARY),
                );
                data_text(ui, value);
                ui.end_row();
            }
        });
}

// ─── Status ───────────────────────────────────────────────────────────────

/// Status pill with an 8px "LED" dot (DESIGN.md "Status Indicators"):
/// translucent status fill, hairline status border, colored label.
pub fn status_chip(ui: &mut Ui, status: t::Status, label: &str) -> Response {
    let fg = status.fg();
    Frame::default()
        .fill(status.bg())
        .stroke(status.border())
        .corner_radius(t::radius::PILL)
        .inner_margin(Margin::symmetric(t::spacing::SM as i8, 2))
        .show(ui, |ui| {
            let (rect, _) = ui.allocate_exact_size(
                egui::vec2(t::spacing::SM, t::spacing::SM),
                egui::Sense::hover(),
            );
            // "Lit" LED: soft halo + solid core.
            ui.painter()
                .circle_filled(rect.center(), 4.0, fg.gamma_multiply(0.35));
            ui.painter().circle_filled(rect.center(), 2.5, fg);
            ui.label(RichText::new(label).small().color(fg));
        })
        .response
}

// ─── Navigation ───────────────────────────────────────────────────────────

/// Rail navigation item. Uses `Button::selected` so assistive tech
/// sees toggle/pressed semantics (not just a colored label).
pub fn nav_item(ui: &mut Ui, selected: bool, label: &str, tooltip: &str) -> Response {
    ui.add(
        egui::Button::new(RichText::new(label).size(t::type_scale::BUTTON.size).color(
            if selected {
                t::accent::PRIMARY
            } else {
                t::text::PRIMARY
            },
        ))
        .selected(selected)
        .corner_radius(t::radius::BASE)
        .fill(if selected {
            t::elevation::FOCUS_RING.color
        } else {
            Color32::TRANSPARENT
        }),
    )
    .on_hover_text(tooltip)
}

// ─── Context patterns ─────────────────────────────────────────────────────

/// Screen header: title + one-line context subtitle. Every screen
/// opens with this so the user always knows where they are and why
/// the surface exists.
pub fn screen_header(ui: &mut Ui, title: &str, subtitle: &str) {
    page_title(ui, title);
    ui.add_space(t::spacing::XS);
    body_muted(ui, subtitle);
    ui.add_space(t::spacing::MD);
}

/// Empty state: centered title + hint (+ optional action). The
/// starfield-free minimal version of the web app's pattern; occupies
/// the remaining vertical space, centered.
pub fn empty_state(
    ui: &mut Ui,
    title: &str,
    hint: &str,
    action_label: Option<&str>,
    on_action: impl FnOnce(),
) {
    let avail = ui.available_size();
    ui.allocate_ui(avail, |ui| {
        ui.vertical_centered(|ui| {
            ui.add_space(t::spacing::XL);
            ui.label(
                RichText::new(title)
                    .size(t::type_scale::HEADLINE.size)
                    .family(t::type_scale::HEADLINE.family.clone())
                    .color(t::text::SECONDARY),
            );
            ui.add_space(t::spacing::SM);
            ui.label(
                RichText::new(hint)
                    .size(t::type_scale::BODY.size)
                    .color(t::border::STRONG),
            );
            if let Some(label) = action_label {
                ui.add_space(t::spacing::MD);
                if ui.button(label).clicked() {
                    on_action();
                }
            }
        });
    });
}

/// Inline error banner (Level-2 surface, error tint border).
pub fn error_banner(ui: &mut Ui, message: &str) {
    Frame::default()
        .fill(t::elevation::LEVEL2_FILL)
        .stroke(t::Status::Error.border())
        .corner_radius(t::radius::MD)
        .inner_margin(Margin::symmetric(
            t::spacing::MD as i8,
            t::spacing::SM as i8,
        ))
        .show(ui, |ui| {
            ui.label(RichText::new(message).color(t::Status::Error.fg()));
        });
}

/// Tooltip convention helper: attach explanatory hover text to any
/// response. Keeps tooltip tone consistent (body size, muted color is
/// handled by egui's tooltip frame).
pub fn with_tooltip(response: Response, text: &str) -> Response {
    response.on_hover_text(text)
}

/// Focus ring helper: stroke a response's rect with the Solar Flare
/// focus ring when egui reports keyboard focus. Call after creating
/// custom widgets that don't get it for free.
pub fn paint_focus_ring(ui: &mut Ui, response: &Response) {
    if response.has_focus() {
        ui.painter().rect_stroke(
            response.rect,
            CornerRadius::same(4),
            t::elevation::FOCUS_RING,
            egui::StrokeKind::Outside,
        );
    }
}
