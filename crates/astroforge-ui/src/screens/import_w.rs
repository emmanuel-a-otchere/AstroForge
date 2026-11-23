//! Import workspace — scan a folder of FITS frames, register them as
//! a session, run the understanding pass, and confirm (the CR-04
//! wizard, condensed into one workflow surface).
//!
//! Flow: pick folder -> scan -> create session + register assets ->
//! analyse (target / capture kind / narrowband + confidences) ->
//! optional override -> confirm.

use astroforge_core::import_scan::ExtractedMetadata;
use egui::{RichText, Ui};

use crate::api::{export, import, quality};
use crate::app::AstroForgeApp;
use crate::semantics as s;
use crate::tokens as t;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Step {
    Pick,
    Scanned,
    Understood,
    Confirmed,
}

pub fn draw(app: &mut AstroForgeApp, ui: &mut Ui) {
    let Some(project) = app.project_context.clone() else {
        return;
    };

    s::screen_header(ui, Screen::ImportW.label(), Screen::ImportW.subtitle());

    if let Some(err) = app.error.take() {
        s::error_banner(ui, &err);
        ui.add_space(t::spacing::SM);
    }

    // Drop intake: a folder dropped this frame becomes the source.
    let dropped = crate::dialogs::dropped_files(ui.ctx())
        .into_iter()
        .find(|p| p.is_dir());
    if let Some(dir) = dropped {
        app.import_source_dir = Some(dir.display().to_string());
        scan_dir(app, &project.project_id);
    }

    match app.import_step {
        Step::Pick => draw_pick(app, ui),
        Step::Scanned => draw_scanned(app, ui),
        Step::Understood => draw_understood(app, ui),
        Step::Confirmed => draw_confirmed(app, ui),
    }
}

use crate::app::Screen;

fn draw_pick(app: &mut AstroForgeApp, ui: &mut Ui) {
    s::section(ui, "Source folder", |ui| {
        s::body_muted(
            ui,
            "Pick the folder holding your FITS lights (plus darks, flats, \
             biases). AstroForge scans it, classifies every frame, and \
             detects the target.",
        );
        ui.add_space(t::spacing::SM);
        ui.horizontal(|ui| {
            if ui
                .button("Choose folder...")
                .on_hover_text("Open a folder picker")
                .clicked()
            {
                if let Some(dir) = crate::dialogs::pick_directory("Select FITS source folder") {
                    app.import_source_dir = Some(dir.display().to_string());
                }
            }
            if let Some(dir) = &app.import_source_dir {
                s::data_text(ui, dir);
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.add_enabled_ui(app.import_source_dir.is_some(), |ui| {
                    if ui.button("Scan folder").clicked() {
                        if let Some(project) = app.project_context.clone() {
                            scan_dir(app, &project.project_id);
                        }
                    }
                });
            });
        });
        ui.add_space(t::spacing::XS);
        s::body_muted(ui, "You can also drag a folder onto the window.");
    });
}

fn scan_dir(app: &mut AstroForgeApp, project_id: &str) {
    let Some(dir) = app.import_source_dir.clone() else {
        return;
    };
    match export::ingest_scan_directory(&dir) {
        Ok(frames) if frames.is_empty() => {
            app.error = Some(format!("No FITS frames found in {dir}"));
        }
        Ok(frames) => {
            app.import_frames = Some(frames);
            app.import_session_id = None;
            match export::session_create(&app.services, project_id, Some(&dir), "normal") {
                Ok(session_id) => {
                    app.import_session_id = Some(session_id);
                    app.import_step = Step::Scanned;
                }
                Err(e) => app.error = Some(e),
            }
        }
        Err(e) => app.error = Some(e),
    }
}

fn draw_scanned(app: &mut AstroForgeApp, ui: &mut Ui) {
    let Some(frames) = app.import_frames.clone() else {
        return;
    };

    s::section(ui, "Scan results", |ui| {
        let lights = frames.iter().filter(|f| f.frame_type == "light").count();
        let anomalies = frames.iter().filter(|f| !f.anomalies.is_empty()).count();
        ui.label(
            RichText::new(format!(
                "{} frames scanned \u{00b7} {} lights \u{00b7} {} with anomalies",
                frames.len(),
                lights,
                anomalies
            ))
            .size(t::type_scale::DATA.size)
            .family(t::type_scale::DATA.family.clone())
            .color(t::text::SECONDARY),
        );
        ui.add_space(t::spacing::SM);
        egui::ScrollArea::vertical()
            .max_height(220.0)
            .show(ui, |ui| {
                egui::Grid::new("import_scan_grid")
                    .num_columns(5)
                    .spacing([t::spacing::MD, t::spacing::XS])
                    .striped(true)
                    .show(ui, |ui| {
                        s::label_caps(ui, "File");
                        s::label_caps(ui, "Type");
                        s::label_caps(ui, "Exp (s)");
                        s::label_caps(ui, "Filter");
                        s::label_caps(ui, "Notes");
                        ui.end_row();
                        for f in &frames {
                            ui.label(RichText::new(file_name(&f.path)).small());
                            s::status_chip(ui, frame_type_status(&f.frame_type), &f.frame_type);
                            ui.label(
                                RichText::new(
                                    f.exptime.map(|v| format!("{v:.0}")).unwrap_or_default(),
                                )
                                .small()
                                .color(t::text::SECONDARY),
                            );
                            ui.label(
                                RichText::new(f.filter.clone().unwrap_or_default())
                                    .small()
                                    .color(t::text::SECONDARY),
                            );
                            ui.label(RichText::new(f.anomalies.join(", ")).small().color(
                                if f.anomalies.is_empty() {
                                    t::border::STRONG
                                } else {
                                    t::Status::Warning.fg()
                                },
                            ));
                            ui.end_row();
                        }
                    });
            });
        ui.add_space(t::spacing::MD);
        ui.horizontal(|ui| {
            if ui
                .button("Register & analyse")
                .on_hover_text(
                    "Create the session, persist every frame as a source asset, \
                     then run the understanding pass",
                )
                .clicked()
            {
                if let Some(project) = app.project_context.clone() {
                    analyse(app, &project.project_id);
                }
            }
            if ui.button("Cancel").clicked() {
                reset_import(app);
            }
        });
    });
}

fn analyse(app: &mut AstroForgeApp, project_id: &str) {
    let Some(frames) = app.import_frames.clone() else {
        return;
    };
    let Some(session_id) = app.import_session_id.clone() else {
        return;
    };

    let frame_inputs = frames
        .iter()
        .map(|f| import::ImportFrameInput {
            path: f.path.clone(),
            frame_type: f.frame_type.clone(),
            exptime: f.exptime,
            filter: f.filter.clone(),
            width: f.width,
            height: f.height,
        })
        .collect();

    let asset_inputs = frames
        .iter()
        .map(|f| import::AssetMetadataInput {
            path: f.path.clone(),
            metadata: ExtractedMetadata {
                exptime: f.exptime,
                filter: f.filter.clone(),
                xbinning: f.binning,
                ybinning: f.binning,
                naxis1: f.width,
                naxis2: f.height,
                ..Default::default()
            },
        })
        .collect();

    match import::import_register_assets(
        &app.services,
        session_id.clone(),
        project_id.to_string(),
        frame_inputs,
    ) {
        Ok(registered) => app.import_registered = Some(registered),
        Err(e) => {
            app.error = Some(e);
            return;
        }
    }

    match import::import_analyse_session(
        &app.services,
        project_id.to_string(),
        session_id,
        asset_inputs,
        None,
    ) {
        Ok(result) => {
            app.import_analysis = Some(result);
            app.import_step = Step::Understood;
        }
        Err(e) => app.error = Some(e),
    }
}

fn draw_understood(app: &mut AstroForgeApp, ui: &mut Ui) {
    let Some(result) = app.import_analysis.clone() else {
        return;
    };

    s::section(ui, "Understanding", |ui| {
        let analysis = &result.analysis;
        ui.label(
            RichText::new(format!(
                "Capture: {:?}  \u{00b7}  narrowband: {:?}  \u{00b7}  confidence {:.0}%",
                analysis.capture.kind,
                analysis.narrowband.composition_suggestion,
                analysis.aggregate_confidence * 100.0
            ))
            .size(t::type_scale::DATA.size)
            .family(t::type_scale::DATA.family.clone()),
        );
        ui.add_space(t::spacing::SM);
        if analysis.aggregate_confidence < 0.6 {
            s::body_muted(
                ui,
                "Low confidence — check the classification above and override \
                 it from Recipes if AstroForge guessed wrong.",
            );
            ui.add_space(t::spacing::SM);
        }
        let quality_req = crate::api::quality::SessionQualityRequest {
            frame_paths: app
                .import_frames
                .as_ref()
                .map(|fs| fs.iter().map(|f| f.path.clone()).collect())
                .unwrap_or_default(),
        };
        match quality::session_quality_report(&app.services, quality_req) {
            Ok(report) => {
                s::status_chip(
                    ui,
                    if report.unreadable.is_empty() {
                        t::Status::Success
                    } else {
                        t::Status::Warning
                    },
                    &format!("{} unreadable frames", report.unreadable.len()),
                )
                .on_hover_text("Frames the scanner could not read; they are excluded");
            }
            Err(_) => {
                s::status_chip(ui, t::Status::Neutral, "quality report unavailable")
                    .on_hover_text("Session quality preflight is unavailable for this session");
            }
        }
    });
    ui.add_space(t::spacing::MD);

    s::section(ui, "Confirm import", |ui| {
        if let Some(registered) = &app.import_registered {
            ui.label(
                RichText::new(format!(
                    "{} frames registered, {} duplicates skipped",
                    registered.registered, registered.duplicates
                ))
                .small()
                .color(t::text::SECONDARY),
            );
        }
        ui.add_space(t::spacing::SM);
        ui.horizontal(|ui| {
            if ui
                .button("Confirm import")
                .on_hover_text("Persist the classification and finish the import")
                .clicked()
            {
                if let Some(session_id) = app.import_session_id.clone() {
                    match import::import_confirm(&app.services, session_id) {
                        Ok(_) => app.import_step = Step::Confirmed,
                        Err(e) => app.error = Some(e),
                    }
                }
            }
            if ui.button("Start over").clicked() {
                reset_import(app);
            }
        });
    });
}

fn draw_confirmed(app: &mut AstroForgeApp, ui: &mut Ui) {
    s::empty_state(
        ui,
        "Session imported",
        "The frames are registered and classified. Head to Process to run \
         the pipeline, or import another folder.",
        None,
        || {},
    );
    if ui.button("Import another folder").clicked() {
        reset_import(app);
    }
    ui.add_space(t::spacing::SM);
    if ui.button("Go to Process").clicked() {
        app.navigate(Screen::Process);
    }
}

fn reset_import(app: &mut AstroForgeApp) {
    app.import_step = crate::screens::import_w::Step::Pick;
    app.import_frames = None;
    app.import_session_id = None;
    app.import_analysis = None;
    app.import_registered = None;
}

fn file_name(path: &str) -> String {
    path.rsplit('/').next().unwrap_or(path).to_string()
}

fn frame_type_status(raw: &str) -> t::Status {
    match raw {
        "light" => t::Status::Info,
        _ => t::Status::Neutral,
    }
}
