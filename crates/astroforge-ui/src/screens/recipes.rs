//! Recipes screen — the library of reproducible pipeline profiles:
//! browse, import/export as JSON, apply to the open project, delete
//! (gated by the confirm modal).

use egui::{RichText, Ui};

use crate::app::{AstroForgeApp, ConfirmAction};
use crate::dialogs::{self, Confirm};
use crate::semantics as s;
use crate::tokens as t;

pub fn draw(app: &mut AstroForgeApp, ui: &mut Ui) {
    s::screen_header(ui, "Recipe Library", Screen::Recipes.subtitle());

    if let Some(err) = app.error.take() {
        s::error_banner(ui, &err);
        ui.add_space(t::spacing::SM);
    }

    // Lazy load + reload after mutations.
    if app.recipes_cache.is_none() {
        app.recipes_cache = crate::api::recipes::recipe_list(&app.services).ok();
    }

    s::section(ui, "Library", |ui| {
        ui.horizontal(|ui| {
            if ui
                .button("Import recipe...")
                .on_hover_text("Load a recipe JSON exported from another machine")
                .clicked()
            {
                if let Some(path) = dialogs::pick_open_file("Import recipe JSON", &["json"]) {
                    match dialogs::read_text(&path)
                        .and_then(|json| crate::api::recipes::recipe_import(&app.services, &json))
                    {
                        Ok(summary) => {
                            app.recipes_toast =
                                Some(format!("Imported \u{201c}{}\u{201d}", summary.name));
                            app.recipes_cache = None;
                        }
                        Err(e) => app.error = Some(e),
                    }
                }
            }
            if ui
                .button("Refresh")
                .on_hover_text("Re-read the library")
                .clicked()
            {
                app.recipes_cache = None;
            }
            if let Some(toast) = &app.recipes_toast {
                s::status_chip(ui, t::Status::Success, toast);
            }
        });
        ui.add_space(t::spacing::SM);

        let recipes = app.recipes_cache.clone().unwrap_or_default();
        if recipes.is_empty() {
            s::body_muted(
                ui,
                "No recipes in the library yet. Seeded profiles appear on \
                 first launch; save a pipeline from Enhance to add more.",
            );
        } else {
            egui::Grid::new("recipes_grid")
                .num_columns(5)
                .spacing([t::spacing::LG, t::spacing::SM])
                .striped(true)
                .show(ui, |ui| {
                    s::label_caps(ui, "Name");
                    s::label_caps(ui, "Target");
                    s::label_caps(ui, "Version");
                    s::label_caps(ui, "Kind");
                    s::label_caps(ui, "Actions");
                    ui.end_row();
                    for r in &recipes {
                        ui.horizontal(|ui| {
                            s::data_text(ui, &r.name);
                            if r.is_system {
                                s::status_chip(ui, t::Status::Info, "system")
                                    .on_hover_text("Seeded profile; cannot be modified");
                            }
                        });
                        ui.label(
                            RichText::new(&r.target_type)
                                .small()
                                .color(t::text::SECONDARY),
                        );
                        ui.label(
                            RichText::new(format!("v{}", r.version))
                                .small()
                                .color(t::text::SECONDARY),
                        );
                        ui.label(
                            RichText::new(&r.description)
                                .small()
                                .color(t::text::SECONDARY),
                        );
                        ui.horizontal(|ui| {
                            if ui
                                .small_button("Export")
                                .on_hover_text("Write the recipe JSON next to this window")
                                .clicked()
                            {
                                let file_name =
                                    format!("{}_v{}.recipe.json", r.profile_id, r.version);
                                if let Some(path) = dialogs::pick_save_file(
                                    "Export recipe JSON",
                                    &file_name,
                                    &["json"],
                                ) {
                                    match crate::api::recipes::recipe_export(
                                        &app.services,
                                        &r.profile_id,
                                        Some(r.version),
                                    )
                                    .and_then(|json| dialogs::write_text(&path, &json))
                                    {
                                        Ok(()) => {
                                            app.recipes_toast =
                                                Some(format!("Exported to {}", path.display()))
                                        }
                                        Err(e) => app.error = Some(e),
                                    }
                                }
                            }
                            let open_project: Option<crate::app::ProjectContext> =
                                app.project_context.clone();
                            ui.add_enabled_ui(open_project.is_some(), |ui| {
                                if ui
                                    .small_button("Apply")
                                    .on_hover_text(
                                        "Apply this recipe to the open project's pipeline plan",
                                    )
                                    .clicked()
                                {
                                    if let (Some(_project), Some(_session)) =
                                        (open_project, app.import_session_id.clone())
                                    {
                                        match crate::api::recipes::recipe_apply(
                                            &app.services,
                                            &r.profile_id,
                                            r.version,
                                            None,
                                        ) {
                                            Ok(applied) => {
                                                app.recipes_toast = Some(format!(
                                                    "Applied \u{201c}{}\u{201d} ({} stages)",
                                                    r.name,
                                                    applied.stages.len()
                                                ));
                                            }
                                            Err(e) => app.error = Some(e),
                                        }
                                    } else {
                                        app.error = Some(
                                            "Open a project first, then apply recipes.".into(),
                                        );
                                    }
                                }
                            });
                            if ui
                                .small_button("Delete")
                                .on_hover_text("Permanently delete this recipe")
                                .clicked()
                            {
                                app.pending_confirm = Some((
                                    Confirm::new(
                                        "Delete recipe",
                                        &format!(
                                            "Permanently delete \u{201c}{}\u{201d} (all versions) \
                                             from the library?",
                                            r.name
                                        ),
                                        "Delete forever",
                                        true,
                                    ),
                                    ConfirmAction::DeleteRecipe {
                                        profile_id: r.profile_id.clone(),
                                    },
                                ));
                            }
                        });
                        ui.end_row();
                    }
                });
        }
    });
}

use crate::app::Screen;
