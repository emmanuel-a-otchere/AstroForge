//! Gallery screen — the collection of finished images across
//! projects, with palette/status metadata (read surface; editing
//! arrives with the P4 polish pass).

use egui::Ui;

use crate::api::gallery;
use crate::app::AstroForgeApp;
use crate::semantics as s;
use crate::tokens as t;

pub fn draw(app: &mut AstroForgeApp, ui: &mut Ui) {
    s::screen_header(
        ui,
        "Gallery",
        "Every finished image AstroForge has produced, across projects.",
    );

    if let Some(err) = app.error.take() {
        s::error_banner(ui, &err);
        ui.add_space(t::spacing::SM);
    }

    s::section(ui, "Collection", |ui| {
        match gallery::gallery_list(&app.services) {
            Ok(items) if items.is_empty() => {
                s::empty_state(
                    ui,
                    "The gallery is empty",
                    "Finished versions land here once you export them.",
                    None,
                    || {},
                );
            }
            Ok(items) => {
                egui::Grid::new("gallery_grid")
                    .num_columns(5)
                    .spacing([t::spacing::LG, t::spacing::SM])
                    .striped(true)
                    .show(ui, |ui| {
                        s::label_caps(ui, "Name");
                        s::label_caps(ui, "Target");
                        s::label_caps(ui, "Integration");
                        s::label_caps(ui, "Status");
                        s::label_caps(ui, "Updated");
                        ui.end_row();
                        for item in &items {
                            s::data_text(ui, &item.name);
                            ui.label(
                                RichText::new(&item.target)
                                    .small()
                                    .color(t::text::SECONDARY),
                            );
                            ui.label(
                                RichText::new(format!("{:.1} h", item.integration_hours))
                                    .small()
                                    .color(t::text::SECONDARY),
                            );
                            gallery_status_chip(ui, &item.status);
                            ui.label(
                                RichText::new(&item.updated_at)
                                    .small()
                                    .color(t::text::SECONDARY),
                            );
                            ui.end_row();
                        }
                    });
            }
            Err(e) => s::error_banner(ui, &format!("Gallery unavailable: {e}")),
        }
    });
}

fn gallery_status_chip(ui: &mut egui::Ui, status: &astroforge_core::gallery::GalleryStatus) {
    use astroforge_core::gallery::GalleryStatus as G;
    let (status, label) = match status {
        G::Pending => (t::Status::Neutral, "Pending"),
        G::Processing => (t::Status::Processing, "Processing"),
        G::Completed => (t::Status::Success, "Completed"),
    };
    s::status_chip(ui, status, label);
}

use egui::RichText;
