//! `ImageCanvas` — the pan/zoom image viewer component.
//!
//! CPU-path display for slice P1: owns the egui texture, handles
//! wheel-zoom (about the pointer) and drag-pan, double-click to
//! refit. Framed with the design-system canvas treatment
//! (darkest neutral fill, hairline border, sharp corners per
//! DESIGN.md's technical-graph rule). A DATA-token statusline reports
//! dimensions and zoom, and the surface carries an accessible name.

use egui::{
    Color32, ColorImage, Frame, Margin, Rect, Response, Sense, Stroke, TextureHandle,
    TextureOptions, Ui, Vec2,
};

use crate::tokens as t;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CanvasState {
    /// Display scale relative to fit-to-canvas (1.0 = fit).
    pub zoom: f32,
    /// Offset of the image center from the canvas center, in points.
    pub pan: Vec2,
}

impl Default for CanvasState {
    fn default() -> Self {
        Self {
            zoom: 1.0,
            pan: Vec2::ZERO,
        }
    }
}

pub struct ImageCanvas {
    handle: Option<TextureHandle>,
    pending: Option<ColorImage>,
    pending_rev: u64,
    uploaded_rev: u64,
    pub state: CanvasState,
}

impl ImageCanvas {
    pub fn new() -> Self {
        Self {
            handle: None,
            pending: None,
            pending_rev: 0,
            uploaded_rev: 0,
            state: CanvasState::default(),
        }
    }

    /// Supply a new display image. `rev` should change whenever the
    /// pixels change (e.g. tone-map toggle) so the texture uploads
    /// exactly once per revision.
    pub fn set_image(&mut self, image: ColorImage, rev: u64) {
        if rev != self.pending_rev {
            self.handle = None; // re-upload lazily inside `show`
        }
        self.pending = Some(image);
        self.pending_rev = rev;
    }

    /// Reset to fit (zoom 1, no pan).
    pub fn refit(&mut self) {
        self.state = CanvasState::default();
    }

    pub fn show(&mut self, ui: &mut Ui, label: &str) -> Response {
        // Lazy upload (needs the ctx).
        if self.handle.is_none() {
            if let Some(image) = self.pending.take() {
                self.handle = Some(ui.ctx().load_texture(
                    format!("image-canvas-{label}"),
                    image,
                    TextureOptions::LINEAR,
                ));
                self.uploaded_rev = self.pending_rev;
            }
        }
        let _ = self.uploaded_rev;

        let desired = Vec2::new(ui.available_width(), ui.available_height());
        let (canvas_rect, _) = ui.allocate_exact_size(desired, Sense::hover());
        let painter = ui.painter_at(canvas_rect);

        // Canvas treatment: darkest neutral + hairline border + sharp
        // corners (technical surface per DESIGN.md "Shapes").
        painter.rect_filled(canvas_rect, t::radius::SM, t::surface::CONTAINER_LOWEST);
        painter.rect_stroke(
            canvas_rect,
            t::radius::SM,
            Stroke::new(t::elevation::LEVEL1_BORDER.width, t::border::HAIRLINE),
            egui::StrokeKind::Inside,
        );

        let mut state = self.state;
        let response = ui
            .interact(
                canvas_rect,
                ui.scope_id().with("image_canvas_drag"),
                Sense::click_and_drag(),
            )
            .on_hover_text("Scroll to zoom, drag to pan, double-click to refit");

        if let Some(handle) = &self.handle {
            let size = Vec2::new(handle.size()[0] as f32, handle.size()[1] as f32);
            let fit = (canvas_rect.width() / size.x)
                .min(canvas_rect.height() / size.y)
                .clamp(f32::EPSILON, 1.0);
            let scale = fit * state.zoom;
            let display = size * scale;

            // Wheel / pinch = zoom about the pointer (keep the
            // image-space point under the cursor fixed); drag = pan;
            // double-click = refit.
            let zoom_factor = ui.input(|i| i.zoom_delta());
            let scrolled = ui.input(|i| i.smooth_scroll_delta.y);
            let factor = if zoom_factor != 1.0 && response.hovered() {
                zoom_factor
            } else if scrolled != 0.0 && response.hovered() {
                (1.0 + scrolled / 400.0).clamp(0.5, 2.0)
            } else {
                1.0
            };
            if factor != 1.0 {
                if let Some(pointer) = response.hover_pos() {
                    let center = canvas_rect.center();
                    let v = center + state.pan;
                    state.pan = pointer + (v - pointer) * factor - center;
                }
                state.zoom = (state.zoom * factor).clamp(0.05, 64.0);
            }
            if response.dragged() {
                state.pan += response.drag_delta();
            }
            if response.double_clicked() {
                state = CanvasState::default();
            }

            // Keep the image from being dragged fully out of view.
            let max_offset = (display * 0.5).max(Vec2::splat(t::spacing::SM));
            state.pan = state.pan.clamp(-max_offset, max_offset);

            let image_rect = Rect::from_center_size(canvas_rect.center() + state.pan, display);
            painter.image(
                handle.id(),
                image_rect,
                Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                Color32::WHITE,
            );

            // Focus ring on hover (Solar Flare hairline, per DESIGN.md
            // "Active Focus").
            if response.hovered() {
                painter.rect_stroke(
                    canvas_rect,
                    t::radius::SM,
                    t::elevation::FOCUS_RING,
                    egui::StrokeKind::Inside,
                );
            }

            // Statusline inside the canvas: dims + zoom (DATA token).
            let status = format!(
                "{} \u{00d7}{} px   {:.0}%",
                handle.size()[0],
                handle.size()[1],
                state.zoom * 100.0
            );
            painter.text(
                canvas_rect.left_bottom() + egui::vec2(t::spacing::SM, -t::spacing::SM),
                egui::Align2::LEFT_BOTTOM,
                status,
                t::type_scale::DATA.font_id(),
                t::text::SECONDARY,
            );
        } else {
            painter.text(
                canvas_rect.center(),
                egui::Align2::CENTER_CENTER,
                "No image",
                t::type_scale::BODY.font_id(),
                t::border::STRONG,
            );
        }

        self.state = state;

        // Rich semantics: the surface reads as an image with usage
        // hints for assistive tech.
        response.widget_info(|| {
            egui::WidgetInfo::labeled(
                egui::accesskit::Role::Image,
                ui.is_enabled(),
                format!("{label} — scroll to zoom, drag to pan, double-click to refit"),
            )
        });

        response
    }
}

impl Default for ImageCanvas {
    fn default() -> Self {
        Self::new()
    }
}

/// Framed wrapper used by screens: reserves `height` points and
/// renders the canvas inside a Level-1 section with a caption.
pub fn framed_canvas(
    ui: &mut Ui,
    caption: &str,
    height: f32,
    show_canvas: impl FnOnce(&mut Ui) -> Response,
) -> Response {
    Frame::default()
        .fill(t::elevation::LEVEL1_FILL)
        .stroke(t::elevation::LEVEL1_BORDER)
        .corner_radius(t::radius::LG)
        .inner_margin(Margin::same(t::spacing::MD as i8))
        .show(ui, |ui| {
            crate::semantics::label_caps(ui, caption);
            ui.add_space(t::spacing::XS);
            ui.allocate_ui(Vec2::new(ui.available_width(), height), show_canvas)
                .inner
        })
        .response
}
