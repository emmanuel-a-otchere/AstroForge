//! `AstroForgeApp` — egui shell: left nav rail + central screens.
//!
//! P0 scope: Home / Projects (functional: list + create) / Recipes /
//! Settings / Help placeholders. Screens are assembled exclusively
//! from `semantics` components and `tokens` — no inline colors or
//! sizes — and every screen opens with a context subtitle so the user
//! always knows where they are.

use std::sync::{Arc, Mutex};

/// Shared slot for a background job's single result.
pub(crate) type JobSlot<T> = Arc<Mutex<Option<Result<T, String>>>>;

use egui::{CentralPanel, ColorImage, Frame, Key, Margin, Panel, RichText, TextEdit, Ui};

use crate::api::project::{self, ProjectSummary};
use crate::dialogs::{self, Confirm, ConfirmOutcome};
use crate::image_canvas::ImageCanvas;
use crate::semantics as s;
use crate::services::Services;
use crate::tokens as t;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    // Global surfaces
    Home,
    Projects,
    Recipes,
    Gallery,
    Settings,
    Help,
    // Project workspaces (reachable when a project is open)
    Overview,
    ImportW,
    Process,
    Enhance,
    Compare,
    Export,
}

impl Screen {
    /// Global nav (always visible).
    const NAV: [(Screen, &'static str, &'static str); 6] = [
        (Screen::Home, "Home", "Overview and recent activity"),
        (
            Screen::Projects,
            "Projects",
            "Your astrophotography projects",
        ),
        (Screen::Recipes, "Recipes", "Reproducible pipeline profiles"),
        (
            Screen::Gallery,
            "Gallery",
            "Finished images across projects",
        ),
        (
            Screen::Settings,
            "Settings",
            "Data location and app information",
        ),
        (Screen::Help, "Help", "Guides and keyboard shortcuts"),
    ];

    /// Project workspaces (visible while a project is open), in
    /// workflow order.
    const WORKSPACES: [(Screen, &'static str, &'static str); 6] = [
        (
            Screen::Overview,
            "Overview",
            "Session, run, and version summary",
        ),
        (
            Screen::ImportW,
            "Import",
            "Bring FITS frames in as a session",
        ),
        (
            Screen::Process,
            "Process",
            "Run the pipeline over the session",
        ),
        (
            Screen::Enhance,
            "Enhance",
            "Arrange stages on the node canvas",
        ),
        (Screen::Compare, "Compare", "Judge versions side by side"),
        (Screen::Export, "Export", "Produce final files"),
    ];

    pub(crate) fn is_workspace(self) -> bool {
        Self::WORKSPACES.iter().any(|(s, _, _)| *s == self)
    }

    pub(crate) fn label(self) -> &'static str {
        Self::NAV
            .iter()
            .chain(Self::WORKSPACES.iter())
            .find(|(screen, _, _)| *screen == self)
            .map(|(_, label, _)| *label)
            .expect("screen is in NAV or WORKSPACES")
    }

    /// Workflow context: why this surface exists (shown under the
    /// title on every screen).
    pub(crate) fn subtitle(self) -> &'static str {
        match self {
            Screen::Home => "Raw smart-telescope data in, publication-ready deep-sky images out.",
            Screen::Projects => {
                "Each project is a self-contained folder holding sessions, \
                 versions, and the decisions you made along the way."
            }
            Screen::Recipes => {
                "Recipes capture a full processing pipeline so any session \
                 can be reprocessed or shared with identical settings."
            }
            Screen::Gallery => {
                "The finished collection \u{2014} every image produced across projects."
            }
            Screen::Settings => {
                "Your data lives in one folder shared by both front-ends \
                 during the migration window."
            }
            Screen::Help => {
                "Navigation: Cmd/Ctrl + 1..5 switches screens, Esc returns \
                 to Home. Workspaces open from a project."
            }
            Screen::Overview => "Where this project stands end to end.",
            Screen::ImportW => "Scan a folder, classify frames, confirm the session.",
            Screen::Process => "Pick a plan, run it, watch every stage.",
            Screen::Enhance => "The pipeline as a node column you can rearrange.",
            Screen::Compare => "Two versions, one verdict.",
            Screen::Export => "Write the finished image out in every format.",
        }
    }

    /// Cmd/Ctrl shortcut ordinal (1-based) for quick navigation.
    pub(crate) fn shortcut(self) -> Key {
        match self {
            Screen::Home => Key::Num1,
            Screen::Projects => Key::Num2,
            Screen::Recipes => Key::Num3,
            Screen::Gallery => Key::Num6,
            Screen::Settings => Key::Num4,
            Screen::Help => Key::Num5,
            Screen::Overview => Key::Num1,
            Screen::ImportW => Key::Num2,
            Screen::Process => Key::Num3,
            Screen::Enhance => Key::Num4,
            Screen::Compare => Key::Num5,
            Screen::Export => Key::Num6,
        }
    }
}

/// The open-project context gating the workspaces.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProjectContext {
    pub project_id: String,
    pub slug: String,
    pub name: String,
}

pub struct AstroForgeApp {
    pub(crate) services: Arc<Services>,
    pub(crate) screen: Screen,
    /// Cached project list + its error state (reloaded on demand).
    pub(crate) projects: Option<Result<Vec<ProjectSummary>, String>>,
    pub(crate) projects_loaded: bool,
    pub(crate) new_project_name: String,
    pub(crate) error: Option<String>,
    /// P1 sample view: synthetic frame through the render pipeline.
    /// NOT user data — proves the display path without artifacts.
    pub(crate) sample_canvas: ImageCanvas,
    pub(crate) sample_linear: Option<ColorImage>,
    pub(crate) sample_stretched: Option<ColorImage>,
    pub(crate) sample_autostretch: bool,
    /// f32 sources for the wgpu upload path (P1b).
    pub(crate) sample_linear_f32: Option<astroforge_core::image::F32Image>,
    pub(crate) sample_stretched_f32: Option<astroforge_core::image::F32Image>,
    /// P2 chrome: pending destructive-action confirmation.
    pub(crate) pending_confirm: Option<(Confirm, ConfirmAction)>,
    /// Open project gating the workspaces (None = global screens).
    pub(crate) project_context: Option<ProjectContext>,
    // ── Import workspace state ──
    pub(crate) import_source_dir: Option<String>,
    pub(crate) import_frames: Option<Vec<crate::api::export::IngestFrameDto>>,
    pub(crate) import_session_id: Option<String>,
    pub(crate) import_registered: Option<crate::api::import::RegisteredAssetsDto>,
    pub(crate) import_analysis: Option<crate::api::import::ImportAnalysisResult>,
    pub(crate) import_step: crate::screens::import_w::Step,
    // ── Process workspace state ──
    pub(crate) process_plans:
        Option<Vec<astroforge_core::pipeline_plans_store::PipelinePlanSummary>>,
    pub(crate) process_selected_plan: Option<String>,
    pub(crate) process_executions: Option<Vec<crate::api::pipeline_plan::StageExecutionSummary>>,
    pub(crate) process_recommendations: Option<Vec<crate::api::pipeline_plan::RecommendationDto>>,
    pub(crate) process_outcome: Option<String>,
    pub(crate) process_job: Option<JobSlot<String>>,
    /// Handle for background-run repaints.
    pub(crate) egui_ctx: Option<egui::Context>,
    // ── Enhance workspace state ──
    pub(crate) enhance_plan: Option<astroforge_core::domain::PipelinePlan>,
    pub(crate) enhance_catalog: Option<astroforge_core::node_catalog::NodeCatalog>,
    pub(crate) enhance_selected: Option<String>,
    // ── Compare workspace state ──
    pub(crate) compare_versions: Option<Vec<crate::api::versions::ImageVersionDto>>,
    pub(crate) compare_version_a: Option<String>,
    pub(crate) compare_version_b: Option<String>,
    pub(crate) compare_image_a: Option<ColorImage>,
    pub(crate) compare_image_b: Option<ColorImage>,
    pub(crate) compare_diff_image: Option<ColorImage>,
    pub(crate) compare_metrics: Option<crate::api::comparison::VersionMetricsComparisonDto>,
    pub(crate) compare_canvas_a: Option<ImageCanvas>,
    pub(crate) compare_canvas_b: Option<ImageCanvas>,
    pub(crate) compare_canvas_diff: Option<ImageCanvas>,
    pub(crate) compare_rev: u64,
    // ── Recipes / gallery state ──
    pub(crate) recipes_cache: Option<Vec<astroforge_core::recipe_store::RecipeSummary>>,
    pub(crate) recipes_toast: Option<String>,
    // ── AI models (Settings) ──
    pub(crate) models_cache: Option<Vec<crate::api::ai::AiModelInfoDto>>,
    pub(crate) models_toast: Option<String>,
    pub(crate) models_job: Option<JobSlot<String>>,
    // ── Export workspace state ──
    pub(crate) export_pixels: Option<astroforge_core::image::F32Image>,
    pub(crate) export_source_label: Option<String>,
    pub(crate) export_formats: crate::screens::export_w::FormatFlags,
    pub(crate) export_out_dir: Option<std::path::PathBuf>,
    pub(crate) export_job: Option<JobSlot<Vec<String>>>,
    pub(crate) export_outputs: Option<Vec<String>>,
}

/// Irreversible actions gated by the confirm modal.
#[derive(Clone, Debug)]
pub enum ConfirmAction {
    DeleteProject(String),
    ArchiveProject(String),
    RemoveStage { plan_id: String, stage_id: String },
    DeleteRecipe { profile_id: String },
}

impl AstroForgeApp {
    pub fn new(cc: &eframe::CreationContext<'_>, services: Arc<Services>) -> Self {
        crate::fonts::install(&cc.egui_ctx);
        crate::theme::install(&cc.egui_ctx);

        // Render-pipeline sample: synthetic field -> tone map ->
        // display images (computed once; the canvas toggles between
        // them).
        let sample = astroforge_render::demo::synthetic_deep_sky(512, 384);
        let sample_linear = Some(astroforge_render::to_color_image(
            &astroforge_render::ToneMap::None.apply(&sample).0,
        ));
        let sample_stretched = Some(astroforge_render::to_color_image(
            &astroforge_render::ToneMap::AutoStretch.apply(&sample).0,
        ));

        // P1b: attach the wgpu display pipeline when eframe's wgpu
        // renderer is active; the canvas then paints rgba16float
        // textures via custom paint callbacks (CPU fallback keeps
        // working otherwise).
        let mut sample_canvas = ImageCanvas::new();
        if let Some(rs) = &cc.wgpu_render_state {
            let ctx = astroforge_render::GpuContext::from_render_state(rs);
            log::info!("wgpu display path active on: {}", ctx.adapter_info);
            let pipeline = astroforge_render::ImagePipeline::new(&ctx);
            sample_canvas.attach_gpu(pipeline, ctx);
        }

        // Keep tone-mapped f32 sources for the GPU upload path.
        let (sample_linear_f32, _) = astroforge_render::ToneMap::None.apply(&sample);
        let (sample_stretched_f32, _) = astroforge_render::ToneMap::AutoStretch.apply(&sample);

        AstroForgeApp {
            services,
            screen: Screen::Home,
            projects: None,
            projects_loaded: false,
            new_project_name: String::new(),
            error: None,
            sample_canvas,
            sample_linear,
            sample_stretched,
            sample_autostretch: true,
            sample_linear_f32: Some(sample_linear_f32),
            sample_stretched_f32: Some(sample_stretched_f32),
            pending_confirm: None,
            project_context: None,
            import_source_dir: None,
            import_frames: None,
            import_session_id: None,
            import_registered: None,
            import_analysis: None,
            import_step: crate::screens::import_w::Step::Pick,
            process_plans: None,
            process_selected_plan: None,
            process_executions: None,
            process_recommendations: None,
            process_outcome: None,
            process_job: None,
            egui_ctx: Some(cc.egui_ctx.clone()),
            enhance_plan: None,
            enhance_catalog: None,
            enhance_selected: None,
            compare_versions: None,
            compare_version_a: None,
            compare_version_b: None,
            compare_image_a: None,
            compare_image_b: None,
            compare_diff_image: None,
            compare_metrics: None,
            compare_canvas_a: None,
            compare_canvas_b: None,
            compare_canvas_diff: None,
            compare_rev: 0,
            recipes_cache: None,
            recipes_toast: None,
            models_cache: None,
            models_toast: None,
            models_job: None,
            export_pixels: None,
            export_source_label: None,
            export_formats: crate::screens::export_w::FormatFlags::default(),
            export_out_dir: None,
            export_job: None,
            export_outputs: None,
        }
    }

    fn reload_projects(&mut self) {
        self.projects = Some(project::project_list(&self.services));
        self.projects_loaded = true;
    }

    /// Global workflow shortcuts: Cmd/Ctrl + 1..5 switch global
    /// screens; Cmd/Ctrl+Shift + 1..6 switch workspaces (when a
    /// project is open); Esc returns Home (closing nothing).
    fn handle_screen_shortcuts(&mut self, ui: &mut Ui) {
        let (cmd, shift) = ui.input(|i| (i.modifiers.command, i.modifiers.shift));
        for (screen, _, _) in Screen::NAV {
            let hit = ui.input(|i| i.key_pressed(screen.shortcut()));
            if hit && cmd && !shift {
                self.navigate(screen);
            }
        }
        if self.project_context.is_some() {
            for (screen, _, _) in Screen::WORKSPACES {
                let hit = ui.input(|i| i.key_pressed(screen.shortcut()));
                if hit && cmd && shift {
                    self.navigate(screen);
                }
            }
        }
        if ui.input(|i| i.key_pressed(Key::Escape)) {
            self.navigate(Screen::Home);
        }
    }

    pub(crate) fn navigate(&mut self, screen: Screen) {
        if screen.is_workspace() && self.project_context.is_none() {
            return; // workspaces live inside an open project
        }
        if self.screen != screen {
            self.screen = screen;
            self.error = None;
        }
    }

    /// Open a project and land on its Overview workspace.
    pub fn open_project_context(&mut self, ctx: ProjectContext) {
        self.project_context = Some(ctx);
        self.navigate(Screen::Overview);
    }

    /// Close the open project, returning to Projects.
    pub fn close_project_context(&mut self) {
        self.project_context = None;
        self.navigate(Screen::Projects);
    }

    fn draw_nav(&mut self, ui: &mut Ui) {
        ui.add_space(t::spacing::MD);
        ui.label(
            RichText::new("AstroForge")
                .size(t::type_scale::HEADLINE.size * 0.75)
                .family(t::type_scale::HEADLINE.family.clone())
                .color(t::accent::PRIMARY),
        );
        ui.add_space(t::spacing::LG);
        if let Some(project) = &self.project_context {
            s::label_caps(ui, "Project");
            ui.label(
                RichText::new(&project.name)
                    .size(t::type_scale::BODY.size)
                    .color(t::text::PRIMARY),
            );
            ui.add_space(t::spacing::XS);
            for (screen, label, tooltip) in Screen::WORKSPACES {
                let selected = self.screen == screen;
                if s::nav_item(ui, selected, label, tooltip).clicked() {
                    self.navigate(screen);
                }
                ui.add_space(t::spacing::XS);
            }
            ui.add_space(t::spacing::SM);
            if ui
                .small_button("Close project")
                .on_hover_text("Return to the project list")
                .clicked()
            {
                self.close_project_context();
            }
            ui.add_space(t::spacing::LG);
            s::label_caps(ui, "General");
        }
        for (screen, label, tooltip) in Screen::NAV {
            let shortcut_ordinal = match screen {
                Screen::Home => "1",
                Screen::Projects => "2",
                Screen::Recipes => "3",
                Screen::Gallery => "6",
                Screen::Settings => "4",
                Screen::Help => "5",
                _ => "?",
            };
            let tooltip = format!("{tooltip}  (Cmd/Ctrl+{shortcut_ordinal})");
            if s::nav_item(ui, self.screen == screen, label, &tooltip).clicked() {
                self.navigate(screen);
            }
            ui.add_space(t::spacing::XS);
        }

        // Pin the footer to the bottom of the rail by consuming the
        // remaining vertical space (with_layout is gone in egui 0.37).
        let footer_height = t::type_scale::METADATA.size * 1.6;
        ui.allocate_at_least(
            egui::vec2(0.0, (ui.available_height() - footer_height).max(0.0)),
            egui::Sense::hover(),
        );
        ui.label(
            RichText::new("egui migration - P0 shell")
                .small()
                .color(t::border::STRONG),
        );
    }

    fn draw_home(&mut self, ui: &mut Ui) {
        s::screen_header(ui, "Welcome to AstroForge", Screen::Home.subtitle());
        if !self.projects_loaded {
            self.reload_projects();
        }
        let project_count = self
            .projects
            .as_ref()
            .and_then(|r| r.as_ref().ok())
            .map(|p| p.len())
            .unwrap_or(0);
        ui.label(
            RichText::new(format!("{project_count} project(s) on disk"))
                .size(t::type_scale::DATA.size)
                .family(t::type_scale::DATA.family.clone()),
        );
        ui.add_space(t::spacing::MD);

        // P1 sample view — the render pipeline end-to-end on a
        // synthetic frame (clearly labeled; not user data).
        s::section(ui, "Sample frame \u{2014} synthetic", |ui| {
            ui.horizontal(|ui| {
                s::body_muted(ui, "Tone map:");
                let linear_clicked = ui
                    .selectable_label(!self.sample_autostretch, "Linear")
                    .clicked();
                let stretch_clicked = ui
                    .selectable_label(self.sample_autostretch, "Screen stretch")
                    .clicked();
                if linear_clicked && self.sample_autostretch {
                    self.sample_autostretch = false;
                    self.sample_canvas.refit();
                }
                if stretch_clicked && !self.sample_autostretch {
                    self.sample_autostretch = true;
                    self.sample_canvas.refit();
                }
            });
            let image = if self.sample_autostretch {
                self.sample_stretched.clone()
            } else {
                self.sample_linear.clone()
            };
            if self.sample_canvas.has_gpu() {
                let src = if self.sample_autostretch {
                    self.sample_stretched_f32.clone()
                } else {
                    self.sample_linear_f32.clone()
                };
                if let Some(img) = src {
                    self.sample_canvas
                        .set_image_f32(&img, self.sample_autostretch as u64 + 1);
                }
            } else if let Some(image) = image {
                self.sample_canvas
                    .set_image(image, self.sample_autostretch as u64 + 1);
            }
            // Keep the canvas inside the viewport regardless of what
            // sits above it.
            let canvas_height = (ui.available_height() * 0.45).clamp(160.0, 320.0);
            crate::image_canvas::framed_canvas(ui, "Canvas", canvas_height, |ui| {
                self.sample_canvas.show(ui, "sample")
            });
            ui.add_space(t::spacing::XS);
            s::body_muted(
                ui,
                "Procedural star field through the P1 display pipeline \
                 (decode -> tone map -> texture). Not real session data — \
                 replaces it only until preview artifacts land in P3.",
            );
        });
        ui.add_space(t::spacing::MD);
        s::body_muted(
            ui,
            "This is the native egui shell (migration slice P0). The Tauri \
             app remains the shipping front-end until the workspaces reach \
             parity.",
        );
    }

    fn draw_projects(&mut self, ui: &mut Ui) {
        s::screen_header(ui, Screen::Projects.label(), Screen::Projects.subtitle());
        if let Some(err) = self.error.take() {
            s::error_banner(ui, &err);
            ui.add_space(t::spacing::SM);
        }

        s::section(ui, "New project", |ui| {
            ui.horizontal(|ui| {
                let name_field = ui
                    .add(
                        TextEdit::singleline(&mut self.new_project_name)
                            .desired_width(280.0)
                            .hint_text("Project name"),
                    )
                    .on_hover_text("A short, unique name; the folder slug is derived from it");
                let enter_pressed =
                    name_field.has_focus() && ui.input(|i| i.key_pressed(Key::Enter));
                ui.add_enabled_ui(!self.new_project_name.trim().is_empty(), |ui| {
                    let clicked = ui.button("Create").clicked();
                    if clicked || enter_pressed {
                        let name = std::mem::take(&mut self.new_project_name);
                        match project::project_create(
                            &self.services,
                            name.trim(),
                            None,
                            env!("CARGO_PKG_VERSION"),
                        ) {
                            Ok(_) => self.reload_projects(),
                            Err(e) => self.error = Some(e),
                        }
                    }
                });
            });
        });
        ui.add_space(t::spacing::MD);

        // List.
        if !self.projects_loaded {
            self.reload_projects();
        }
        let projects = self.projects.clone();
        match projects {
            Some(Ok(projects)) if projects.is_empty() => {
                s::empty_state(
                    ui,
                    "No projects yet",
                    "Create your first project above, or open the Tauri app \
                     to import your first session.",
                    None,
                    || {},
                );
            }
            Some(Ok(projects)) => {
                egui::Grid::new("projects_grid")
                    .num_columns(5)
                    .spacing([t::spacing::LG, t::spacing::SM])
                    .striped(true)
                    .show(ui, |ui| {
                        s::label_caps(ui, "Name");
                        s::label_caps(ui, "Status");
                        s::label_caps(ui, "Created");
                        s::label_caps(ui, "Updated");
                        s::label_caps(ui, "Actions");
                        ui.end_row();
                        for p in &projects {
                            s::data_text(ui, &p.name);
                            let (status, label) = status_for_project(&p.status);
                            s::status_chip(ui, status, label)
                                .on_hover_text(format!("Project status: {label}"));
                            ui.label(
                                RichText::new(&p.created_at)
                                    .small()
                                    .color(t::text::SECONDARY),
                            );
                            ui.label(
                                RichText::new(&p.updated_at)
                                    .small()
                                    .color(t::text::SECONDARY),
                            );
                            ui.horizontal(|ui| {
                                if ui
                                    .small_button("Open")
                                    .on_hover_text("Open the project workspaces")
                                    .clicked()
                                {
                                    app_open_row(self, p);
                                }
                                if ui
                                    .small_button("Archive")
                                    .on_hover_text("Move the project folder into the archive")
                                    .clicked()
                                {
                                    self.pending_confirm = Some((
                                        Confirm::new(
                                            "Archive project",
                                            &format!(
                                                "Move \u{201c}{}\u{201d} into the project archive?                                                  It stays on disk and can be recovered.",
                                                p.name
                                            ),
                                            "Archive",
                                            false,
                                        ),
                                        ConfirmAction::ArchiveProject(p.project_id.clone()),
                                    ));
                                }
                                if ui
                                    .small_button("Delete")
                                    .on_hover_text("Permanently remove this project from disk")
                                    .clicked()
                                {
                                    self.pending_confirm = Some((
                                        Confirm::new(
                                            "Delete project",
                                            &format!(
                                                "Permanently delete \u{201c}{}\u{201d} and all of                                                  its sessions, versions, and exports from disk?                                                  This cannot be undone.",
                                                p.name
                                            ),
                                            "Delete forever",
                                            true,
                                        ),
                                        ConfirmAction::DeleteProject(p.project_id.clone()),
                                    ));
                                }
                            });
                            ui.end_row();
                        }
                    });
            }
            Some(Err(e)) => {
                s::error_banner(ui, &format!("Failed to load projects: {e}"));
            }
            None => unreachable!("loaded above"),
        }
        ui.add_space(t::spacing::SM);
        if ui
            .button("Reload")
            .on_hover_text("Re-read the global project registry from disk")
            .clicked()
        {
            self.reload_projects();
        }
    }

    fn draw_placeholder(&mut self, ui: &mut Ui, slice: &str) {
        s::screen_header(ui, self.screen.label(), self.screen.subtitle());
        s::empty_state(
            ui,
            "On its way",
            &format!("This surface arrives in {slice}."),
            None,
            || {},
        );
    }

    fn draw_settings(&mut self, ui: &mut Ui) {
        s::screen_header(ui, Screen::Settings.label(), Screen::Settings.subtitle());
        s::section(ui, "Data", |ui| {
            let data_dir = self.services.data_dir.display().to_string();
            let projects_root = self.services.projects_root.display().to_string();
            let toolkit = format!(
                "astroforge-ui {} (egui/eframe 0.37)",
                env!("CARGO_PKG_VERSION")
            );
            s::kv_table(
                ui,
                &[
                    ("Data directory", data_dir.as_str()),
                    ("Projects root", projects_root.as_str()),
                    ("UI toolkit", toolkit.as_str()),
                ],
            );
            ui.add_space(t::spacing::SM);
            s::body_muted(
                ui,
                "The data directory is shared with the Tauri front-end \
                 during the migration window.",
            );
        });
        ui.add_space(t::spacing::MD);
        draw_ai_models(self, ui);
    }
}

fn draw_ai_models(app: &mut AstroForgeApp, ui: &mut Ui) {
    use crate::api::ai;

    // Drain finished download jobs.
    if let Some(job) = app.models_job.clone() {
        let finished = job.lock().expect("model job").take();
        if let Some(result) = finished {
            match result {
                Ok(path) => app.models_toast = Some(format!("installed: {path}")),
                Err(e) => app.error = Some(e),
            }
            app.models_job = None;
            app.models_cache = None; // refresh installed flags
        }
    }
    if app.models_cache.is_none() {
        app.models_cache = Some(ai::ai_model_list());
    }

    s::section(ui, "AI models", |ui| {
        for m in app.models_cache.clone().unwrap_or_default() {
            ui.horizontal(|ui| {
                s::status_chip(
                    ui,
                    if m.installed {
                        t::Status::Success
                    } else {
                        t::Status::Neutral
                    },
                    if m.installed {
                        "installed"
                    } else {
                        "available"
                    },
                );
                ui.vertical(|ui| {
                    ui.label(
                        RichText::new(format!("{} v{}", m.name, m.version))
                            .small()
                            .color(t::text::PRIMARY),
                    );
                    ui.label(
                        RichText::new(format!(
                            "{} \u{00b7} {} \u{00b7} {:.0} MB",
                            m.stage,
                            m.license,
                            m.size_bytes as f64 / 1_048_576.0
                        ))
                        .small()
                        .color(t::border::STRONG),
                    );
                });
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if !m.installed && m.download_url.is_some() {
                        ui.add_enabled_ui(app.models_job.is_none(), |ui| {
                            if ui
                                .small_button("Download")
                                .on_hover_text("Fetch and install the model artifact")
                                .clicked()
                            {
                                let name = m.name.clone();
                                let slot: JobSlot<String> = Arc::new(Mutex::new(None));
                                let slot_thread = slot.clone();
                                let ctx = app
                                    .egui_ctx
                                    .clone()
                                    .expect("egui context captured at startup");
                                std::thread::spawn(move || {
                                    let result = ai::ai_model_download(&name);
                                    *slot_thread.lock().expect("model job") = Some(result);
                                    ctx.request_repaint();
                                });
                                app.models_job = Some(slot);
                            }
                        });
                    }
                });
            });
            ui.add_space(t::spacing::XS);
        }
        if let Some(toast) = &app.models_toast {
            ui.add_space(t::spacing::XS);
            s::status_chip(ui, t::Status::Success, toast);
        }
    });
}

/// Workflow vocabulary: domain lifecycle -> UI status semantics.
/// Keeps the mapping in one audited place instead of scattered
/// matches across screens.
pub fn status_for_project(
    status: &astroforge_core::domain::ProjectStatus,
) -> (t::Status, &'static str) {
    use astroforge_core::domain::ProjectStatus as P;
    match status {
        P::New | P::Imported | P::Recovering => (t::Status::Neutral, "New"),
        P::Analyzing | P::Processing | P::Enhancing => (t::Status::Processing, "Processing"),
        P::Ready | P::Processed => (t::Status::Success, "Ready"),
        P::Paused => (t::Status::Warning, "Paused"),
        P::Failed => (t::Status::Error, "Failed"),
        P::Review => (t::Status::Info, "Review"),
        P::Exported => (t::Status::Success, "Exported"),
    }
}

impl eframe::App for AstroForgeApp {
    fn ui(&mut self, ui: &mut Ui, _frame: &mut eframe::Frame) {
        self.handle_screen_shortcuts(ui);

        // Window title tracks the active screen (workflow context).
        ui.ctx()
            .send_viewport_cmd(egui::ViewportCommand::Title(format!(
                "AstroForge \u{2014} {}",
                self.screen.label()
            )));

        Panel::left("nav")
            .exact_size(220.0)
            .resizable(false)
            .frame(
                Frame::default()
                    .fill(t::surface::CONTAINER_LOWEST)
                    .inner_margin(Margin::same(t::spacing::MD as i8)),
            )
            .show(ui, |ui| self.draw_nav(ui));

        CentralPanel::default()
            .frame(
                Frame::default()
                    .fill(t::surface::BASE)
                    .inner_margin(Margin::same(t::spacing::CONTAINER_SAFE as i8)),
            )
            .show(ui, |ui| match self.screen {
                Screen::Home => self.draw_home(ui),
                Screen::Projects => self.draw_projects(ui),
                Screen::Recipes => crate::screens::recipes::draw(self, ui),
                Screen::Gallery => crate::screens::gallery::draw(self, ui),
                Screen::Settings => self.draw_settings(ui),
                Screen::Help => self.draw_placeholder(ui, "P4 (help)"),
                Screen::Overview => crate::screens::overview::draw(self, ui),
                Screen::ImportW => crate::screens::import_w::draw(self, ui),
                Screen::Process => crate::screens::process::draw(self, ui),
                Screen::Enhance => crate::screens::enhance::draw(self, ui),
                Screen::Compare => crate::screens::compare::draw(self, ui),
                Screen::Export => crate::screens::export_w::draw(self, ui),
            });

        egui::Panel::bottom("statusbar")
            .exact_size(t::spacing::LG)
            .frame(
                Frame::default()
                    .fill(t::surface::CONTAINER_LOWEST)
                    .inner_margin(Margin::symmetric(
                        t::spacing::MD as i8,
                        t::spacing::XS as i8,
                    )),
            )
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(format!("astroforge-ui {}", env!("CARGO_PKG_VERSION")))
                            .small()
                            .color(t::border::STRONG),
                    );
                    ui.label(
                        RichText::new("Cmd/Ctrl+1..5 screens \u{00b7} Esc home")
                            .small()
                            .color(t::border::STRONG),
                    );
                });
            });

        // P2 chrome: destructive-action gate (drawn over everything).
        if let Some((_confirm, _action)) = self.pending_confirm.as_ref() {
            let confirm = _confirm.clone();
            let action = _action.clone();
            match dialogs::confirm_modal(ui.ctx(), &confirm) {
                ConfirmOutcome::Pending => {}
                ConfirmOutcome::Confirmed => {
                    self.run_confirmed(action);
                    self.pending_confirm = None;
                }
                ConfirmOutcome::Dismissed => {
                    self.pending_confirm = None;
                }
            }
        }
    }
}

impl AstroForgeApp {
    /// Execute a confirmed destructive action.
    fn run_confirmed(&mut self, action: ConfirmAction) {
        match action {
            ConfirmAction::DeleteProject(slug) => {
                if let Err(e) = project::project_delete(&self.services, &slug, true) {
                    self.error = Some(e);
                }
                self.reload_projects();
            }
            ConfirmAction::ArchiveProject(slug) => {
                if let Err(e) = project::project_archive(&self.services, &slug) {
                    self.error = Some(e);
                }
                self.reload_projects();
            }
            ConfirmAction::RemoveStage { plan_id, stage_id } => {
                if let Err(e) =
                    crate::api::node_graph::remove_stage(&self.services, plan_id, stage_id)
                {
                    self.error = Some(e);
                }
                self.enhance_plan = None; // resync on next frame
            }
            ConfirmAction::DeleteRecipe { profile_id } => {
                if let Err(e) = crate::api::recipes::recipe_delete(&self.services, &profile_id) {
                    self.error = Some(e);
                }
                self.recipes_cache = None;
            }
        }
    }
}

/// Open a project row into its workspace context.
fn app_open_row(app: &mut AstroForgeApp, summary: &ProjectSummary) {
    match project::open_project_by_id(&app.services, &summary.project_id) {
        Ok(_) => {
            app.open_project_context(ProjectContext {
                project_id: summary.project_id.clone(),
                slug: summary.project_id.clone(),
                name: summary.name.clone(),
            });
        }
        Err(e) => app.error = Some(e),
    }
}
