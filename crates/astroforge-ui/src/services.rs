//! `Services` — the egui-side counterpart of the Tauri managed state
//! (`src-tauri/src/main.rs` `setup` closure). Same stores, same file
//! names, same seed behavior; mutexes instead of `tauri::State`.
//!
//! All mutexes are `Arc`-reachable so later slices can run long
//! pipeline work on background threads and post results back into the
//! egui frame loop.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use astroforge_core::diff_cache::DiffCache;
use astroforge_core::domain_store::DomainStore;
use astroforge_core::gallery::GalleryStore;
use astroforge_core::pipeline_plans_store::PipelinePlanStore;
use astroforge_core::project::ProjectManager;
use astroforge_core::recipe_events::RecipeEventStore;
use astroforge_core::recipe_store::RecipeStore;
use astroforge_core::session::SessionStore;

/// Shared application services handed to screens / api functions.
pub struct Services {
    pub data_dir: PathBuf,
    pub gallery: Arc<Mutex<GalleryStore>>,
    pub sessions: Arc<Mutex<SessionStore>>,
    pub recipes: Arc<Mutex<RecipeStore>>,
    pub recipe_events: Arc<Mutex<RecipeEventStore>>,
    pub diff_cache: Arc<Mutex<DiffCache>>,
    pub project_manager: Arc<Mutex<ProjectManager>>,
    pub domain_store: Arc<Mutex<DomainStore>>,
    pub pipeline_plans: Arc<Mutex<PipelinePlanStore>>,
    pub projects_root: PathBuf,
    pub previews_dir: PathBuf,
}

impl Services {
    /// Open every store exactly like the Tauri `setup` closure so both
    /// front-ends are interchangeable over one data directory.
    pub fn open(data_dir: &Path) -> Result<Services, String> {
        std::fs::create_dir_all(data_dir)
            .map_err(|e| format!("failed to create data dir {}: {e}", data_dir.display()))?;

        let gallery = GalleryStore::new(&data_dir.join("gallery.sqlite"))
            .map_err(|e| format!("failed to open gallery store: {e}"))?;

        let sessions = SessionStore::new(&data_dir.join("sessions.sqlite"))
            .map_err(|e| format!("failed to open session store: {e}"))?;

        let recipe_path = data_dir.join("recipes.sqlite");
        let recipes = RecipeStore::new(&recipe_path)
            .map_err(|e| format!("failed to open recipe store: {e}"))?;
        // Same first-launch seeding as Tauri (DwarfII v1 + M42-Natural-v1).
        recipes
            .seed_if_empty()
            .map_err(|e| format!("failed to seed recipe store: {e}"))?;

        let recipe_events = RecipeEventStore::new(&recipe_path)
            .map_err(|e| format!("failed to open recipe event store: {e}"))?;

        let projects_root = data_dir.join("projects");
        std::fs::create_dir_all(&projects_root)
            .map_err(|e| format!("failed to create projects root: {e}"))?;
        let project_db = projects_root.join("projects.db");
        let domain_store = DomainStore::new(&project_db)
            .map_err(|e| format!("failed to open project store: {e}"))?;

        let pipeline_plans = PipelinePlanStore::new(data_dir.join("pipeline_plans.sqlite"))
            .map_err(|e| format!("failed to open pipeline plans store: {e}"))?;

        Ok(Services {
            data_dir: data_dir.to_path_buf(),
            gallery: Arc::new(Mutex::new(gallery)),
            sessions: Arc::new(Mutex::new(sessions)),
            recipes: Arc::new(Mutex::new(recipes)),
            recipe_events: Arc::new(Mutex::new(recipe_events)),
            diff_cache: Arc::new(Mutex::new(DiffCache::new())),
            project_manager: Arc::new(Mutex::new(ProjectManager::new(projects_root.clone()))),
            domain_store: Arc::new(Mutex::new(domain_store)),
            pipeline_plans: Arc::new(Mutex::new(pipeline_plans)),
            projects_root,
            previews_dir: data_dir.join("previews"),
        })
    }
}
