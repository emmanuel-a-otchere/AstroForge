//! Enhance workspace — the pipeline as a vertical node column you can
//! rearrange (constrained-mode node editor; graph truth stays in
//! astroforge-core). Palette on the left, canvas center, recipe
//! hand-off below.

use astroforge_core::domain::PipelinePlan;
use egui::{RichText, Ui};

use crate::api::node_graph as ng;
use crate::app::AstroForgeApp;
use crate::node_canvas as geom;
use crate::semantics as s;
use crate::tokens as t;

pub fn draw(app: &mut AstroForgeApp, ui: &mut Ui) {
    s::screen_header(ui, Screen::Enhance.label(), Screen::Enhance.subtitle());

    if let Some(err) = app.error.take() {
        s::error_banner(ui, &err);
        ui.add_space(t::spacing::SM);
    }

    // The plan is shared with Process (same selection).
    let Some(plan_id) = app.process_selected_plan.clone() else {
        s::body_muted(ui, "Select or create a pipeline plan in Process first.");
        if ui.button("Go to Process").clicked() {
            app.navigate(Screen::Process);
        }
        return;
    };

    // One-time catalog load.
    if app.enhance_catalog.is_none() {
        app.enhance_catalog = ng::read_node_catalog().ok();
    }

    // Load / re-sync the plan after every mutation.
    if app.enhance_plan.is_none() {
        app.enhance_plan =
            crate::api::pipeline_plan::plan_for_canvas(&app.services, plan_id.clone()).ok();
    }
    let Some(plan) = app.enhance_plan.clone() else {
        s::error_banner(ui, "Could not load the plan.");
        return;
    };

    let mut plan_changed: Option<PipelinePlan> = None;

    ui.columns(2, |columns| {
        // ─── Palette ────────────────────────────────────────────────
        let palette_ui = &mut columns[0];
        s::section(palette_ui, "Stage palette", |ui| {
            let Some(catalog) = app.enhance_catalog.clone() else {
                s::body_muted(ui, "Catalog unavailable.");
                return;
            };
            egui::ScrollArea::vertical()
                .max_height(300.0)
                .show(ui, |ui| {
                    for entry in &catalog.entries {
                        let response = ui
                            .horizontal(|ui| {
                                s::status_chip(
                                    ui,
                                    if entry.ai_uses_ai {
                                        t::Status::Info
                                    } else {
                                        t::Status::Neutral
                                    },
                                    if entry.ai_uses_ai { "AI" } else { "std" },
                                );
                                ui.label(
                                    RichText::new(&entry.label).small().color(t::text::PRIMARY),
                                );
                            })
                            .response
                            .on_hover_text(&entry.description);
                        if response.clicked() {
                            match ng::insert_stage(
                                &app.services,
                                ng::InsertStageRequest {
                                    plan_id: plan_id.clone(),
                                    stage_type: entry.stage_type.clone(),
                                },
                            ) {
                                Ok(updated) => plan_changed = Some(updated),
                                Err(e) => app.error = Some(e),
                            }
                        }
                    }
                });
        });

        // ─── Node canvas ────────────────────────────────────────────
        let canvas_ui = &mut columns[1];
        s::section(canvas_ui, "Pipeline column", |ui| {
            let stages = &plan.stages;
            if stages.is_empty() {
                s::empty_state(
                    ui,
                    "Empty pipeline",
                    "Insert stages from the palette on the left.",
                    None,
                    || {},
                );
                return;
            }
            let count = stages.len();
            let column = geom::column_size(count);
            let desired = egui::vec2(
                ui.available_width(),
                (column.y + t::spacing::LG).min(ui.available_height()),
            );
            let (canvas_rect, _) = ui.allocate_exact_size(desired, egui::Sense::hover());
            let painter = ui.painter_at(canvas_rect);
            painter.rect_filled(canvas_rect, t::radius::SM, t::surface::CONTAINER_LOWEST);
            painter.rect_stroke(
                canvas_rect,
                t::radius::SM,
                egui::Stroke::new(t::elevation::LEVEL1_BORDER.width, t::border::HAIRLINE),
                egui::StrokeKind::Inside,
            );

            let origin = canvas_rect.left_top() + egui::vec2(t::spacing::MD, t::spacing::MD);
            ui.interact(
                canvas_rect,
                ui.scope_id().with("node_canvas"),
                egui::Sense::click_and_drag(),
            );

            // Wires first (consecutive stages), then cards on top.
            for i in 0..count.saturating_sub(1) {
                let from = origin
                    + geom::card_position(i)
                    + egui::vec2(geom::NODE_WIDTH / 2.0, geom::NODE_HEIGHT);
                let to =
                    origin + geom::card_position(i + 1) + egui::vec2(geom::NODE_WIDTH / 2.0, 0.0);
                painter.line_segment([from, to], egui::Stroke::new(1.0, t::accent::TERTIARY_DEEP));
            }

            let mut clicked_index: Option<usize> = None;
            let mut toggle_index: Option<usize> = None;
            let mut remove_index: Option<usize> = None;

            for (i, stage) in stages.iter().enumerate() {
                let card_pos = origin + geom::card_position(i);
                let rect = egui::Rect::from_min_size(
                    card_pos,
                    egui::vec2(geom::NODE_WIDTH, geom::NODE_HEIGHT),
                );
                let selected = app.enhance_selected.as_deref() == Some(&stage.stage_id);
                let enabled = stage.enabled;

                let (fill, text_color, wire_alpha) = if selected {
                    (t::surface::CONTAINER_HIGHEST, t::text::PRIMARY, 1.0)
                } else if enabled {
                    (t::surface::CONTAINER, t::text::PRIMARY, 1.0)
                } else {
                    (t::surface::CONTAINER_LOW, t::border::STRONG, 0.45)
                };
                painter.rect_filled(rect, t::radius::BASE, fill);
                painter.rect_stroke(
                    rect,
                    t::radius::BASE,
                    egui::Stroke::new(
                        1.0,
                        if selected {
                            t::accent::PRIMARY
                        } else {
                            t::border::HAIRLINE
                        },
                    ),
                    egui::StrokeKind::Inside,
                );
                painter.text(
                    rect.left_top() + egui::vec2(8.0, 14.0),
                    egui::Align2::LEFT_TOP,
                    &stage.label,
                    t::type_scale::METADATA.font_id(),
                    text_color,
                );
                painter.text(
                    rect.left_bottom() + egui::vec2(8.0, -10.0),
                    egui::Align2::LEFT_BOTTOM,
                    format!("#{} {}", stage.sequence, stage.stage_type),
                    t::type_scale::LABEL_CAPS.font_id(),
                    t::border::STRONG,
                );

                // Card-local interactions via a per-card interact.
                let card_response = ui.interact(
                    rect,
                    ui.scope_id().with(format!("node_card_{}", stage.stage_id)),
                    egui::Sense::click_and_drag(),
                );
                if card_response.clicked() {
                    clicked_index = Some(i);
                }
                if card_response.double_clicked() {
                    toggle_index = Some(i);
                }
                if card_response.secondary_clicked() {
                    remove_index = Some(i);
                }

                // Drag reorder: while dragging this card, draw it at
                // the pointer; on release, resolve the drop index.
                if card_response.dragged() {
                    if let Some(pointer) = card_response.interact_pointer_pos() {
                        let local = pointer - origin;
                        if let Some(target) = geom::drop_index(count, local.y) {
                            let hover_rect = egui::Rect::from_min_size(
                                origin + geom::card_position(target),
                                egui::vec2(geom::NODE_WIDTH, geom::NODE_HEIGHT),
                            );
                            painter.rect_stroke(
                                hover_rect,
                                t::radius::BASE,
                                t::elevation::FOCUS_RING,
                                egui::StrokeKind::Inside,
                            );
                        }
                    }
                }
                if card_response.drag_stopped() {
                    if let Some(pointer) = ui.input(|i| i.pointer.interact_pos()) {
                        let local = pointer - origin;
                        if let Some(target) = geom::drop_index(count, local.y) {
                            if target != i {
                                match ng::reorder_stage(
                                    &app.services,
                                    plan_id.clone(),
                                    stage.stage_id.clone(),
                                    target,
                                ) {
                                    Ok(updated) => plan_changed = Some(updated),
                                    Err(e) => app.error = Some(e),
                                }
                            }
                        }
                    }
                }

                let _ = wire_alpha;
            }

            if let Some(i) = clicked_index {
                app.enhance_selected = Some(stages[i].stage_id.clone());
            }
            if let Some(i) = toggle_index {
                let stage = &stages[i];
                match ng::set_stage_enabled(
                    &app.services,
                    plan_id.clone(),
                    stage.stage_id.clone(),
                    !stage.enabled,
                ) {
                    Ok(updated) => plan_changed = Some(updated),
                    Err(e) => app.error = Some(e),
                }
            }
            if let Some(i) = remove_index {
                let stage = &stages[i];
                app.pending_confirm = Some((
                    s_confirm(format!(
                        "Remove stage \u{201c}{}\u{201d} from the pipeline?",
                        stage.stage_type
                    )),
                    crate::app::ConfirmAction::RemoveStage {
                        plan_id: plan_id.clone(),
                        stage_id: stage.stage_id.clone(),
                    },
                ));
            }
        });
    });
    ui.add_space(t::spacing::MD);

    // Apply mutations AFTER the frame's borrows end.
    if let Some(updated) = plan_changed {
        app.enhance_plan = Some(updated);
    }

    // ─── Recipe hand-off ───────────────────────────────────────────
    s::section(ui, "Recipe hand-off", |ui| {
        ui.horizontal(|ui| {
            if ui
                .button("Save as recipe")
                .on_hover_text("Serialize this graph into a reusable Recipe")
                .clicked()
            {
                match ng::graph_to_recipe(
                    &app.services,
                    plan_id.clone(),
                    env!("CARGO_PKG_VERSION").to_string(),
                ) {
                    Ok(_) => app.process_outcome = Some("Recipe saved from graph.".into()),
                    Err(e) => app.error = Some(e),
                }
            }
            if ui
                .button("Resync from store")
                .on_hover_text("Reload the plan, discarding local view state")
                .clicked()
            {
                app.enhance_plan = None;
            }
        });
        if let Some(outcome) = &app.process_outcome {
            ui.label(RichText::new(outcome).small().color(t::text::SECONDARY));
        }
    });
}

fn s_confirm(message: String) -> Confirm {
    Confirm::new("Remove stage", &message, "Remove", true)
}

use crate::app::Screen;
use crate::dialogs::Confirm;
