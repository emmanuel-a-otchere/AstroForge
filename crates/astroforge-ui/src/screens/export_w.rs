//! Export workspace — write the finished version out in every
//! format via `export_multi_format`, run on a background thread.

use std::sync::{Arc, Mutex};

use crate::app::JobSlot;

use egui::{RichText, Ui};

use crate::app::{AstroForgeApp, Screen};
use crate::semantics as s;
use crate::tokens as t;

/// Which formats to emit (mirrors `core::export::ExportFormat`
/// minus history/sidecar payloads, which the screen fills in).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FormatFlags {
    pub tiff16: bool,
    pub png8: bool,
    pub jpeg8: bool,
    pub fits32: bool,
}

impl Default for FormatFlags {
    fn default() -> Self {
        Self {
            tiff16: true,
            png8: true,
            jpeg8: false,
            fits32: false,
        }
    }
}

pub fn draw(app: &mut AstroForgeApp, ui: &mut Ui) {
    s::screen_header(ui, Screen::Export.label(), Screen::Export.subtitle());

    if let Some(err) = app.error.take() {
        s::error_banner(ui, &err);
        ui.add_space(t::spacing::SM);
    }

    poll_job(app);

    // Source: the newest version (lazy).
    if app.export_pixels.is_none() {
        if let Some(project) = app.project_context.clone() {
            if let Ok(response) =
                crate::api::versions::image_version_list(&app.services, &project.project_id)
            {
                if let Some(latest) = response.versions.last() {
                    if let Ok(img) =
                        crate::api::comparison::version_pixels(&app.services, &latest.version_id)
                    {
                        app.export_source_label =
                            Some(format!("V{} \u{2014} {}", latest.sequence, latest.label));
                        app.export_pixels = Some(img);
                    }
                }
            }
        }
    }

    s::section(ui, "Source", |ui| {
        match (&app.export_pixels, &app.export_source_label) {
            (Some(_), Some(label)) => s::body_muted(ui, &format!("Exporting {label}")),
            _ => {
                s::empty_state(
                    ui,
                    "Nothing to export yet",
                    "Complete a pipeline run first \u{2014} versions appear as they finish.",
                    None,
                    || {},
                );
            }
        }
    });
    ui.add_space(t::spacing::MD);

    s::section(ui, "Formats", |ui| {
        ui.horizontal(|ui| {
            ui.checkbox(&mut app.export_formats.tiff16, "16-bit TIFF");
            ui.checkbox(&mut app.export_formats.png8, "8-bit PNG");
            ui.checkbox(&mut app.export_formats.jpeg8, "8-bit JPEG");
            ui.checkbox(&mut app.export_formats.fits32, "32-bit FITS");
        });
        ui.add_space(t::spacing::SM);
        ui.horizontal(|ui| {
            if ui
                .button("Choose output folder...")
                .on_hover_text("Where the files land")
                .clicked()
            {
                if let Some(dir) = crate::dialogs::pick_directory("Export output folder") {
                    app.export_out_dir = Some(dir);
                }
            }
            if let Some(dir) = &app.export_out_dir {
                s::data_text(ui, &dir.display().to_string());
            }
        });
        ui.add_space(t::spacing::MD);
        let any_format = app.export_formats.tiff16
            || app.export_formats.png8
            || app.export_formats.jpeg8
            || app.export_formats.fits32;
        ui.add_enabled_ui(any_format && app.export_pixels.is_some(), |ui| {
            let busy = app.export_job.is_some();
            ui.add_enabled_ui(!busy, |ui| {
                if ui
                    .button("Export")
                    .on_hover_text("Write the selected formats to the output folder")
                    .clicked()
                {
                    start_export(app);
                }
            });
            if busy {
                ui.add_space(t::spacing::SM);
                ui.add(egui::Spinner::new().size(14.0));
                ui.label(
                    RichText::new("exporting...")
                        .small()
                        .color(t::text::SECONDARY),
                );
            }
        });
    });

    if let Some(outputs) = &app.export_outputs {
        ui.add_space(t::spacing::MD);
        s::section(ui, "Outputs", |ui| {
            for path in outputs {
                s::data_text(ui, path);
            }
            if outputs.is_empty() {
                s::body_muted(ui, "No files written yet.");
            }
        });
    }
}

fn start_export(app: &mut AstroForgeApp) {
    let Some(image) = app.export_pixels.clone() else {
        return;
    };
    let Some(dir) = app.export_out_dir.clone() else {
        app.error = Some("Choose an output folder first.".into());
        return;
    };
    let flags = app.export_formats;
    let label = app
        .export_source_label
        .clone()
        .unwrap_or_else(|| "export".into());
    let safe = label
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '_' })
        .collect::<String>();
    let base = dir.join(safe).display().to_string();
    let channels = image.channels() as u32;
    let mut pixels = Vec::with_capacity(image.len());
    for v in image.iter() {
        pixels.push(*v);
    }

    let mut formats: Vec<astroforge_core::export::ExportFormat> = Vec::new();
    if flags.tiff16 {
        formats.push(astroforge_core::export::ExportFormat::Tiff16);
    }
    if flags.png8 {
        formats.push(astroforge_core::export::ExportFormat::Png8);
    }
    if flags.jpeg8 {
        formats.push(astroforge_core::export::ExportFormat::Jpeg8 { quality: 92 });
    }
    if flags.fits32 {
        formats.push(astroforge_core::export::ExportFormat::Fits32);
    }

    let args = crate::api::export::MultiExportArgs {
        pixels,
        width: image.width() as u32,
        height: image.height() as u32,
        channels,
        base_path: base,
        formats,
        report: None,
    };

    let slot: JobSlot<Vec<String>> = Arc::new(Mutex::new(None));
    let slot_thread = slot.clone();
    let ctx = app
        .egui_ctx
        .clone()
        .expect("egui context captured at startup");
    std::thread::spawn(move || {
        let result = crate::api::export::export_multi_format(args);
        *slot_thread.lock().expect("export slot") = Some(result);
        ctx.request_repaint();
    });
    app.export_job = Some(slot);
}

fn poll_job(app: &mut AstroForgeApp) {
    let Some(job) = app.export_job.clone() else {
        return;
    };
    let finished = job.lock().expect("export slot").take();
    if let Some(result) = finished {
        match result {
            Ok(paths) => app.export_outputs = Some(paths),
            Err(e) => app.error = Some(e),
        }
        app.export_job = None;
    }
}
