//! Compare workspace — two versions, one verdict: A/B canvases,
//! per-pixel diff, and the metric deltas that decide the winner.

use egui::{RichText, Ui};

use crate::app::AstroForgeApp;
use crate::image_canvas::ImageCanvas;
use crate::semantics as s;
use crate::tokens as t;

pub fn draw(app: &mut AstroForgeApp, ui: &mut Ui) {
    let Some(project) = app.project_context.clone() else {
        return;
    };

    s::screen_header(ui, Screen::Compare.label(), Screen::Compare.subtitle());

    if let Some(err) = app.error.take() {
        s::error_banner(ui, &err);
        ui.add_space(t::spacing::SM);
    }

    // Version timeline (lazy).
    if app.compare_versions.is_none() {
        app.compare_versions =
            crate::api::versions::image_version_list(&app.services, &project.project_id)
                .ok()
                .map(|r| r.versions);
    }
    let Some(versions) = app.compare_versions.clone() else {
        s::body_muted(ui, "Version timeline unavailable.");
        return;
    };
    if versions.is_empty() {
        s::empty_state(
            ui,
            "No versions yet",
            "Versions appear after a pipeline run completes (Process).",
            None,
            || {},
        );
        return;
    }

    // ─── Version pickers ───────────────────────────────────────────
    s::section(ui, "Versions", |ui| {
        ui.horizontal(|ui| {
            ui.label("A:");
            version_combo(ui, "cmp_a", &versions, &mut app.compare_version_a);
            ui.label("B:");
            version_combo(ui, "cmp_b", &versions, &mut app.compare_version_b);
            if ui
                .button("Load")
                .on_hover_text("Load both versions into the canvases and compute metrics")
                .clicked()
            {
                load_pair(app);
            }
        });
        let label_of = |id: &Option<String>| {
            id.as_ref()
                .and_then(|id| versions.iter().find(|v| &v.version_id == id))
                .map(|v| format!("V{} \u{2014} {}", v.sequence, v.label))
                .unwrap_or_else(|| "\u{2014} (none)".into())
        };
        s::body_muted(ui, &format!("A = {}", label_of(&app.compare_version_a)));
        s::body_muted(ui, &format!("B = {}", label_of(&app.compare_version_b)));
    });
    ui.add_space(t::spacing::MD);

    // ─── Canvases ──────────────────────────────────────────────────
    if app.compare_canvas_a.is_none() || app.compare_canvas_b.is_none() {
        app.compare_canvas_a = Some(ImageCanvas::new());
        app.compare_canvas_b = Some(ImageCanvas::new());
    }
    ui.columns(2, |columns| {
        columns[0].vertical(|ui| {
            s::label_caps(ui, "Version A");
            if let (Some(canvas), Some(image)) = (&mut app.compare_canvas_a, &app.compare_image_a) {
                canvas.set_image(image.clone(), app.compare_rev);
                canvas.show(ui, "compare-a");
            } else {
                s::body_muted(ui, "Load a version to see it here.");
            }
        });
        columns[1].vertical(|ui| {
            s::label_caps(ui, "Version B");
            if let (Some(canvas), Some(image)) = (&mut app.compare_canvas_b, &app.compare_image_b) {
                canvas.set_image(image.clone(), app.compare_rev);
                canvas.show(ui, "compare-b");
            } else {
                s::body_muted(ui, "Load a version to see it here.");
            }
        });
    });
    ui.add_space(t::spacing::MD);

    // ─── Diff ──────────────────────────────────────────────────────
    if let Some(diff) = &app.compare_diff_image {
        s::section(ui, "Per-pixel diff (amplified)", |ui| {
            let h = (ui.available_height() * 0.3).clamp(120.0, 260.0);
            if let Some(canvas) = &mut app.compare_canvas_diff {
                canvas.set_image(diff.clone(), app.compare_rev);
                canvas.show(ui, "compare-diff");
            }
            let _ = h;
        });
        ui.add_space(t::spacing::MD);
    }

    // ─── Metrics ───────────────────────────────────────────────────
    s::section(ui, "Metric deltas", |ui| match &app.compare_metrics {
        Some(metrics) => {
            egui::Grid::new("compare_metrics")
                .num_columns(3)
                .spacing([t::spacing::LG, t::spacing::XS])
                .striped(true)
                .show(ui, |ui| {
                    s::label_caps(ui, "Metric");
                    s::label_caps(ui, "A \u{2192} B");
                    s::label_caps(ui, "Delta");
                    ui.end_row();
                    for row in &metrics.rows {
                        use astroforge_core::comparison::DeltaDirection as DD;
                        let (color, _tag) = match row.direction {
                            DD::Improved => (t::Status::Success.fg(), "improved"),
                            DD::Degraded => (t::Status::Error.fg(), "degraded"),
                            DD::Unchanged | DD::Inconclusive => (t::border::STRONG, "unchanged"),
                        };
                        ui.label(RichText::new(row.label).small().color(t::text::PRIMARY));
                        ui.label(
                            RichText::new(format!(
                                "{} \u{2192} {} {}",
                                row.baseline_value
                                    .map(fmt2f64)
                                    .unwrap_or_else(|| "\u{2014}".into()),
                                row.compared_value
                                    .map(fmt2f64)
                                    .unwrap_or_else(|| "\u{2014}".into()),
                                row.unit
                            ))
                            .small()
                            .color(t::text::SECONDARY),
                        );
                        ui.label(
                            RichText::new(format!("{:+.1}%", row.percent_change))
                                .small()
                                .color(color),
                        );
                        ui.end_row();
                    }
                });
            ui.add_space(t::spacing::SM);
            s::body_muted(ui, &metrics.summary);
        }
        None => {
            s::body_muted(ui, "Pick two versions and press Load.");
        }
    });
}

fn version_combo(
    ui: &mut Ui,
    salt: &str,
    versions: &[crate::api::versions::ImageVersionDto],
    selected: &mut Option<String>,
) {
    let current = selected
        .as_ref()
        .and_then(|id| versions.iter().find(|v| &v.version_id == id))
        .map(|v| format!("V{} \u{2014} {}", v.sequence, v.label))
        .unwrap_or_else(|| "pick...".into());
    egui::ComboBox::from_id_salt(salt)
        .selected_text(current)
        .show_ui(ui, |ui| {
            for v in versions {
                let label = format!("V{} \u{2014} {}", v.sequence, v.label);
                if ui
                    .selectable_label(selected.as_deref() == Some(&v.version_id), &label)
                    .clicked()
                {
                    *selected = Some(v.version_id.clone());
                }
            }
        });
}

fn load_pair(app: &mut AstroForgeApp) {
    let (Some(a), Some(b)) = (app.compare_version_a.clone(), app.compare_version_b.clone()) else {
        app.error = Some("Pick both versions first.".into());
        return;
    };

    // Pixels via the canonical loader; keep the f32 images for the
    // diff pass and tone-map for display.
    let load_f32 = |id: String| -> Option<astroforge_core::image::F32Image> {
        crate::api::comparison::version_pixels(&app.services, &id).ok()
    };
    let pixels_a = load_f32(a.clone());
    let pixels_b = load_f32(b.clone());
    let load_one = |img: &Option<astroforge_core::image::F32Image>| -> Option<egui::ColorImage> {
        img.as_ref().map(|img| {
            astroforge_render::to_color_image(&astroforge_render::ToneMap::AutoStretch.apply(img).0)
        })
    };
    app.compare_image_a = load_one(&pixels_a);
    app.compare_image_b = load_one(&pixels_b);

    // Diff: core's canonical compute_diff over the two stretched
    // display buffers (amplified mode, like the web CompareTools).
    if let (Some(img_a), Some(img_b)) = (pixels_a, pixels_b) {
        let (stretched_a, _) = astroforge_render::ToneMap::AutoStretch.apply(&img_a);
        let (stretched_b, _) = astroforge_render::ToneMap::AutoStretch.apply(&img_b);
        let bytes = |img: &astroforge_core::image::F32Image| -> Vec<u8> {
            img.iter()
                .map(|v| (v.clamp(0.0, 1.0) * 255.0) as u8)
                .collect()
        };
        let w = img_a.width() as u32;
        let diff_bytes = astroforge_core::difference::compute_diff(
            astroforge_core::difference::DiffKind::Amplified,
            &bytes(&stretched_a),
            &bytes(&stretched_b),
            8.0,
            w,
        );
        // Rebuild a ColorImage from the u8 diff (interleaved pixels).
        let h = stretched_a.height();
        let stride = w as usize * stretched_a.channels();
        let channels = stretched_a.channels();
        let mut color: Vec<egui::Color32> = Vec::with_capacity(w as usize * h);
        for y in 0..h {
            for x in 0..(w as usize) {
                let base = y * stride + x * channels;
                let r = diff_bytes[base];
                let g = diff_bytes[base + (1 % channels)];
                let b = diff_bytes[base + (2 % channels)];
                color.push(egui::Color32::from_rgb(r, g, b));
            }
        }
        app.compare_diff_image = Some(egui::ColorImage::new([w as usize, h], color));
    }

    app.compare_metrics =
        crate::api::comparison::compare_version_metrics(&app.services, &a, &b).ok();
    app.compare_rev += 1;
    if let Some(canvas) = &mut app.compare_canvas_a {
        canvas.refit();
    }
    if let Some(canvas) = &mut app.compare_canvas_b {
        canvas.refit();
    }
}

fn fmt2f64(v: f64) -> String {
    format!("{v:.2}")
}

use crate::app::Screen;
