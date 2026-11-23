//! Overview workspace — where the project stands end to end:
//! §8 checklist, sessions, pipeline runs.

use crate::api::{import, project};
use crate::app::{AstroForgeApp, Screen};
use crate::semantics as s;
use crate::tokens as t;
use egui::Ui;

pub fn draw(app: &mut AstroForgeApp, ui: &mut Ui) {
    let Some(project) = app.project_context.clone() else {
        return;
    };

    s::screen_header(ui, &project.name, Screen::Overview.subtitle());

    if let Some(err) = app.error.take() {
        s::error_banner(ui, &err);
        ui.add_space(t::spacing::SM);
    }

    // ─── §8 checklist ──────────────────────────────────────────────
    s::section(
        ui,
        "Project checklist",
        |ui| match project::project_overview(&app.services, &project.project_id) {
            Ok(overview) => {
                ui.horizontal_wrapped(|ui| {
                    checklist_chip(
                        ui,
                        "Imported",
                        overview.imported,
                        "Source frames are registered in a session",
                    );
                    checklist_chip(
                        ui,
                        "Analyzed",
                        overview.analyzed,
                        "The session classification has completed",
                    );
                    checklist_chip(
                        ui,
                        "Processed",
                        overview.processed,
                        "At least one pipeline run completed",
                    );
                    checklist_chip(
                        ui,
                        "Versioned",
                        overview.versioned,
                        "A final image version exists",
                    );
                    checklist_chip(ui, "Exported", overview.exported, "Files were written out");
                });
            }
            Err(e) => s::error_banner(ui, &format!("Overview unavailable: {e}")),
        },
    );
    ui.add_space(t::spacing::MD);

    // ─── Sessions ──────────────────────────────────────────────────
    s::section(ui, "Sessions", |ui| match import::session_list_for_project(
        &app.services,
        project.project_id.clone(),
    ) {
        Ok(sessions) if sessions.is_empty() => {
            s::body_muted(ui, "No sessions yet — import a folder of FITS frames.");
            ui.add_space(t::spacing::SM);
            if ui.button("Go to Import").clicked() {
                app.navigate(Screen::ImportW);
            }
        }
        Ok(sessions) => {
            egui::Grid::new("overview_sessions")
                .num_columns(4)
                .spacing([t::spacing::LG, t::spacing::SM])
                .striped(true)
                .show(ui, |ui| {
                    s::label_caps(ui, "Name");
                    s::label_caps(ui, "State");
                    s::label_caps(ui, "Source");
                    s::label_caps(ui, "Updated");
                    ui.end_row();
                    for session in &sessions {
                        ui.horizontal(|ui| {
                            s::data_text(ui, &session.name);
                            if session.is_active {
                                s::status_chip(ui, t::Status::Info, "active");
                            }
                        });
                        s::status_chip(
                            ui,
                            session_status(&session.processing_status),
                            &session.processing_status,
                        );
                        ui.label(
                            RichText::new(session.source_location.as_deref().unwrap_or("\u{2014}"))
                                .small()
                                .color(t::text::SECONDARY),
                        );
                        ui.label(
                            RichText::new(&session.updated_at)
                                .small()
                                .color(t::text::SECONDARY),
                        );
                        ui.end_row();
                    }
                });
        }
        Err(e) => s::error_banner(ui, &format!("Sessions unavailable: {e}")),
    });
    ui.add_space(t::spacing::MD);

    // ─── Version timeline (DAG) ───────────────────────────────────
    s::section(
        ui,
        "Version timeline",
        |ui| match crate::api::versions::image_version_list(&app.services, &project.project_id) {
            Ok(response) if !response.versions.is_empty() => {
                let nodes = crate::version_dag::build(&response.versions);
                let layers = crate::version_dag::layers(&nodes);
                let col_w = 170.0;
                let gap_x = 48.0;
                let row_h = 26.0;
                let max_rows = layers.iter().map(|l| l.len()).max().unwrap_or(1);
                let size = egui::vec2(
                    layers.len() as f32 * (col_w + gap_x),
                    max_rows as f32 * row_h + t::spacing::SM,
                );
                let (rect, _) = ui.allocate_exact_size(size, egui::Sense::hover());
                let painter = ui.painter_at(rect);
                for (layer_idx, layer) in layers.iter().enumerate() {
                    for (row_idx, node) in layer.iter().enumerate() {
                        let pos = rect.left_top()
                            + egui::vec2(
                                layer_idx as f32 * (col_w + gap_x),
                                row_idx as f32 * row_h,
                            );
                        let chip_rect =
                            egui::Rect::from_min_size(pos, egui::vec2(col_w, row_h - 4.0));
                        painter.rect_filled(
                            chip_rect,
                            t::radius::BASE,
                            if layer_idx == 0 {
                                t::surface::CONTAINER_HIGH
                            } else {
                                t::surface::CONTAINER
                            },
                        );
                        painter.rect_stroke(
                            chip_rect,
                            t::radius::BASE,
                            egui::Stroke::new(1.0, t::border::HAIRLINE),
                            egui::StrokeKind::Inside,
                        );
                        painter.text(
                            chip_rect.left_center() + egui::vec2(t::spacing::SM, 0.0),
                            egui::Align2::LEFT_CENTER,
                            &node.label,
                            t::type_scale::METADATA.font_id(),
                            t::text::PRIMARY,
                        );
                    }
                }
            }
            Ok(_) => {
                s::body_muted(
                    ui,
                    "No versions yet \u{2014} they appear after a run completes.",
                );
            }
            Err(e) => s::error_banner(ui, &format!("Timeline unavailable: {e}")),
        },
    );
    ui.add_space(t::spacing::MD);

    // ─── Pipeline runs ─────────────────────────────────────────────
    s::section(ui, "Pipeline runs", |ui| {
        match project::pipeline_run_list(&app.services, project.project_id.as_str()) {
            Ok(runs) if runs.is_empty() => {
                s::body_muted(ui, "No pipeline runs yet — start one from Process.");
                ui.add_space(t::spacing::SM);
                if ui.button("Go to Process").clicked() {
                    app.navigate(Screen::Process);
                }
            }
            Ok(runs) => {
                egui::Grid::new("overview_runs")
                    .num_columns(3)
                    .spacing([t::spacing::LG, t::spacing::SM])
                    .striped(true)
                    .show(ui, |ui| {
                        s::label_caps(ui, "Run");
                        s::label_caps(ui, "Status");
                        s::label_caps(ui, "Completed");
                        ui.end_row();
                        for run in &runs {
                            s::data_text(ui, &short_id(&run.run_id));
                            let (status, label) = run_status(&run.status);
                            s::status_chip(ui, status, label)
                                .on_hover_text(format!("Run status: {label}"));
                            ui.label(
                                RichText::new(run.completed_at.as_deref().unwrap_or("\u{2014}"))
                                    .small()
                                    .color(t::text::SECONDARY),
                            );
                            ui.end_row();
                        }
                    });
            }
            Err(e) => s::error_banner(ui, &format!("Runs unavailable: {e}")),
        }
    });
}

use egui::RichText;

fn checklist_chip(ui: &mut Ui, label: &str, done: bool, tooltip: &str) {
    let (status, text) = if done {
        (t::Status::Success, format!("{label} \u{2713}"))
    } else {
        (t::Status::Neutral, format!("{label} pending"))
    };
    s::status_chip(ui, status, &text).on_hover_text(tooltip);
    ui.add_space(t::spacing::SM);
}

fn session_status(raw: &str) -> t::Status {
    match raw {
        "Completed" | "completed" => t::Status::Success,
        "Processing" | "processing" | "Running" => t::Status::Processing,
        "Failed" | "failed" => t::Status::Error,
        "Paused" => t::Status::Warning,
        _ => t::Status::Neutral,
    }
}

fn run_status(status: &astroforge_core::domain::PipelineRunStatus) -> (t::Status, &'static str) {
    use astroforge_core::domain::PipelineRunStatus as R;
    match status {
        R::Queued => (t::Status::Neutral, "Queued"),
        R::Running => (t::Status::Processing, "Running"),
        R::Completed => (t::Status::Success, "Completed"),
        R::Failed => (t::Status::Error, "Failed"),
        R::Cancelled => (t::Status::Neutral, "Cancelled"),
        R::Paused => (t::Status::Warning, "Paused"),
        R::Recovering => (t::Status::Warning, "Recovering"),
    }
}

fn short_id(id: &str) -> String {
    id.chars().take(12).collect()
}
