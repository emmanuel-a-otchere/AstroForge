//! Process workspace — create a plan from the active session, start
//! the run on a background thread, watch stage executions stream in,
//! apply/dismiss recommendations, cancel/pause as needed.

use std::sync::{Arc, Mutex};

use crate::app::JobSlot;

use egui::{RichText, Ui};

use crate::api::pipeline_plan as pp;
use crate::api::project;
use crate::app::AstroForgeApp;
use crate::semantics as s;
use crate::tokens as t;

pub fn draw(app: &mut AstroForgeApp, ui: &mut Ui) {
    let Some(project) = app.project_context.clone() else {
        return;
    };

    s::screen_header(ui, Screen::Process.label(), Screen::Process.subtitle());

    if let Some(err) = app.error.take() {
        s::error_banner(ui, &err);
        ui.add_space(t::spacing::SM);
    }

    poll_run_job(app);

    // ─── Plans ─────────────────────────────────────────────────────
    s::section(ui, "Pipeline plans", |ui| {
        let plans = app.process_plans.clone().unwrap_or_else(|| {
            refresh_plans(app);
            app.process_plans.clone().unwrap_or_default()
        });
        if plans.is_empty() {
            s::body_muted(ui, "No plans yet — create one from the active session.");
        } else {
            for plan in &plans {
                let selected = app.process_selected_plan.as_deref() == Some(&plan.plan_id);
                let label = format!(
                    "{} \u{00b7} {} \u{00b7} {}",
                    short_id(&plan.plan_id),
                    plan.recipe_id.as_deref().unwrap_or("builtin"),
                    plan.status
                );
                if ui
                    .selectable_label(selected, RichText::new(label).monospace())
                    .clicked()
                {
                    app.process_selected_plan = Some(plan.plan_id.clone());
                    refresh_plan_detail(app);
                }
            }
        }
        ui.add_space(t::spacing::SM);
        ui.horizontal(|ui| {
            if ui
                .button("New plan from active session")
                .on_hover_text(
                    "Derive a plan for the project's active session (built-in OSC recipe)",
                )
                .clicked()
            {
                create_plan(app, &project.project_id);
            }
            if ui
                .button("Refresh")
                .on_hover_text("Re-read plans from the store")
                .clicked()
            {
                refresh_plans(app);
            }
        });
    });
    ui.add_space(t::spacing::MD);

    // ─── Run + stages ──────────────────────────────────────────────
    if let Some(plan_id) = app.process_selected_plan.clone() {
        s::section(ui, "Run", |ui| {
            ui.horizontal(|ui| {
                let job_running = app.process_job.is_some();
                ui.add_enabled_ui(!job_running, |ui| {
                    if ui
                        .button("Start run")
                        .on_hover_text("Execute the plan's stages (runs in the background)")
                        .clicked()
                    {
                        start_run(app, plan_id.clone());
                    }
                });
                if ui.button("Pause").clicked() {
                    if let Err(e) = pp::pause_pipeline_run(&app.services, plan_id.clone()) {
                        app.error = Some(e);
                    }
                }
                if ui.button("Resume").clicked() {
                    if let Err(e) = pp::resume_pipeline_run(&app.services, plan_id.clone()) {
                        app.error = Some(e);
                    }
                }
                if ui
                    .button("Cancel")
                    .on_hover_text("Ask the runner to stop after the current stage")
                    .clicked()
                {
                    if let Err(e) = pp::cancel_pipeline_run(&app.services, plan_id.clone()) {
                        app.error = Some(e);
                    }
                }
                if job_running {
                    ui.add_space(t::spacing::SM);
                    ui.add(egui::Spinner::new().size(14.0));
                    ui.label(
                        RichText::new("run in background...")
                            .small()
                            .color(t::text::SECONDARY),
                    );
                }
            });
            if let Some(outcome) = &app.process_outcome {
                ui.add_space(t::spacing::SM);
                ui.label(RichText::new(outcome).small().color(t::text::SECONDARY));
            }
        });
        ui.add_space(t::spacing::MD);

        s::section(ui, "Stage executions", |ui| {
            match app.process_executions.clone() {
                Some(execs) if !execs.is_empty() => {
                    egui::ScrollArea::vertical()
                        .max_height(220.0)
                        .show(ui, |ui| {
                            egui::Grid::new("process_execs")
                                .num_columns(4)
                                .spacing([t::spacing::MD, t::spacing::XS])
                                .striped(true)
                                .show(ui, |ui| {
                                    s::label_caps(ui, "Stage");
                                    s::label_caps(ui, "Status");
                                    s::label_caps(ui, "Attempt");
                                    s::label_caps(ui, "Completed");
                                    ui.end_row();
                                    for exec in &execs {
                                        s::data_text(ui, &exec.stage_id);
                                        s::status_chip(ui, exec_status(&exec.status), &exec.status);
                                        ui.label(
                                            RichText::new(exec.attempt.to_string())
                                                .small()
                                                .color(t::text::SECONDARY),
                                        );
                                        ui.label(
                                            RichText::new(
                                                exec.completed_at.as_deref().unwrap_or("\u{2014}"),
                                            )
                                            .small()
                                            .color(t::text::SECONDARY),
                                        );
                                        ui.end_row();
                                    }
                                });
                        });
                    ui.add_space(t::spacing::SM);
                    ui.horizontal(|ui| {
                        if ui.small_button("Retry last failed").clicked() {
                            if let Some(failed) = execs
                                .iter()
                                .rev()
                                .find(|e| e.status.eq_ignore_ascii_case("failed"))
                            {
                                if let Err(e) = pp::retry_stage(
                                    &app.services,
                                    failed.plan_id.clone(),
                                    failed.stage_id.clone(),
                                    pp::RetryKindDto::AsIs,
                                ) {
                                    app.error = Some(e);
                                }
                                refresh_plan_detail(app);
                            }
                        }
                        if ui.small_button("Skip next").clicked() {
                            if let Some(next) = execs
                                .iter()
                                .find(|e| e.status.eq_ignore_ascii_case("pending"))
                            {
                                if let Err(e) = pp::skip_stage(
                                    &app.services,
                                    next.plan_id.clone(),
                                    next.stage_id.clone(),
                                ) {
                                    app.error = Some(e);
                                }
                                refresh_plan_detail(app);
                            }
                        }
                    });
                }
                _ => {
                    s::body_muted(ui, "No stage executions yet — start a run.");
                }
            }
        });
        ui.add_space(t::spacing::MD);

        // ─── Recommendations ───────────────────────────────────────
        s::section(ui, "Recommendations", |ui| {
            let recs = app.process_recommendations.clone().unwrap_or_default();
            let pending: Vec<_> = recs
                .iter()
                .filter(|r| {
                    r.user_decision.as_deref() != Some("applied")
                        && r.user_decision.as_deref() != Some("dismissed")
                })
                .collect();
            if pending.is_empty() {
                s::body_muted(ui, "No pending recommendations.");
            } else {
                for rec in pending {
                    ui.horizontal(|ui| {
                        s::status_chip(
                            ui,
                            t::Status::Info,
                            &format!("{:.0}%", rec.confidence * 100.0),
                        );
                        ui.label(
                            RichText::new(&rec.evidence_summary)
                                .small()
                                .color(t::text::PRIMARY),
                        );
                    });
                    ui.horizontal(|ui| {
                        if ui.small_button("Apply").clicked() {
                            if let Err(e) = pp::apply_recommendation(&app.services, rec.id.clone())
                            {
                                app.error = Some(e);
                            }
                            refresh_plan_detail(app);
                        }
                        if ui.small_button("Dismiss").clicked() {
                            if let Err(e) =
                                pp::dismiss_recommendation(&app.services, rec.id.clone())
                            {
                                app.error = Some(e);
                            }
                            refresh_plan_detail(app);
                        }
                    });
                    ui.add_space(t::spacing::XS);
                }
            }
        });
    } else {
        s::body_muted(ui, "Select or create a plan above.");
    }
    let _ = project;
}

fn create_plan(app: &mut AstroForgeApp, project_id: &str) {
    // Active session from the global store; import sets it.
    let summary = match project::project_get(&app.services, project_id) {
        Ok(p) => p,
        Err(e) => {
            app.error = Some(e);
            return;
        }
    };
    let Some(session_id) = summary.active_session_id.clone() else {
        app.error = Some("No active session — import a folder first (Cmd/Ctrl+Shift+2).".into());
        return;
    };
    let request = pp::CreatePipelinePlanRequest {
        project_id: project_id.to_string(),
        session_id,
        recipe_id: Some("deep_sky_osc_balanced".into()),
        mode: Some("auto".into()),
        session_understanding: None,
    };
    match pp::create_pipeline_plan(&app.services, request) {
        Ok(plan) => {
            app.process_selected_plan = Some(plan.plan_id.clone());
            refresh_plans(app);
            refresh_plan_detail(app);
        }
        Err(e) => app.error = Some(e),
    }
}

fn refresh_plans(app: &mut AstroForgeApp) {
    let Some(project) = app.project_context.clone() else {
        return;
    };
    match pp::pipeline_plan_list_for_project(&app.services, project.project_id.clone()) {
        Ok(plans) => app.process_plans = Some(plans),
        Err(e) => app.error = Some(e),
    }
}

fn refresh_plan_detail(app: &mut AstroForgeApp) {
    let Some(plan_id) = app.process_selected_plan.clone() else {
        return;
    };
    app.process_executions =
        pp::pipeline_plan_list_stage_executions(&app.services, plan_id.clone()).ok();
    app.process_recommendations = pp::get_recommendations_for_plan(&app.services, plan_id).ok();
}

fn start_run(app: &mut AstroForgeApp, plan_id: String) {
    let services = app.services.clone();
    let slot: JobSlot<String> = Arc::new(Mutex::new(None));
    let slot_for_thread = slot.clone();
    let ctx = ui_ctx(app);
    std::thread::spawn(move || {
        let result = pp::start_pipeline_run(&services, plan_id)
            .map(|outcome| format!("run finished: {:?}", outcome));
        *slot_for_thread.lock().expect("run slot") = Some(result);
        ctx.request_repaint();
    });
    app.process_job = Some(slot);
    app.process_outcome = Some("Run started in the background.".into());
}

/// Drain a finished background run (if any) each frame.
fn poll_run_job(app: &mut AstroForgeApp) {
    let Some(job) = app.process_job.clone() else {
        return;
    };
    let finished = job.lock().expect("run slot").take();
    if let Some(result) = finished {
        match result {
            Ok(text) => app.process_outcome = Some(text),
            Err(e) => app.error = Some(e),
        }
        app.process_job = None;
        refresh_plan_detail(app);
        refresh_plans(app);
    }
}

fn ui_ctx(app: &AstroForgeApp) -> egui::Context {
    // The egui context is available through the frame loop; screens
    // hold it via the app. Simplest stable handle: stash a clone in
    // the app on first use.
    app.egui_ctx
        .clone()
        .expect("egui context captured on first frame")
}

fn exec_status(raw: &str) -> t::Status {
    match raw.to_ascii_lowercase().as_str() {
        "completed" => t::Status::Success,
        "running" | "in_progress" => t::Status::Processing,
        "failed" => t::Status::Error,
        "skipped" => t::Status::Neutral,
        "paused" => t::Status::Warning,
        _ => t::Status::Neutral,
    }
}

fn short_id(id: &str) -> String {
    id.chars().take(12).collect()
}

use crate::app::Screen;
