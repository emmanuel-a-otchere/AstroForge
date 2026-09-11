//! CR-02.4 — Project lifecycle: create / open / rename / archive / delete /
//! recover, over the self-contained per-project directory layout (CR-02 §18).
//!
//! This is the application-service layer of CR-02 §36: it composes the
//! `DomainStore` (SQLite metadata) with the filesystem layout and never lets
//! the UI touch either directly.
//!
//! Layout (an implementation detail — users never need to see it):
//!
//! ```text
//! <projects_root>/<project-slug>/
//! ├── project.db        ← DomainStore (migrated automatically on open, §30)
//! ├── project.json      ← portable manifest (§20): id, name, schema version
//! ├── sources/  artifacts/  previews/  checkpoints/
//! ├── recipes/  exports/  logs/  cache/
//! ```
//!
//! Delete semantics follow CR-02 §26: deletion requires explicit
//! confirmation at the API boundary.

use crate::domain::ProjectStatus;
use crate::domain_store::{DomainStore, DomainStoreError};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Sub-directories of the §18 layout, created on create and repaired on
/// recover.
const LAYOUT_DIRS: &[&str] = &[
    "sources",
    "artifacts",
    "previews",
    "checkpoints",
    "recipes",
    "exports",
    "logs",
    "cache",
];

const DB_FILE: &str = "project.db";
const MANIFEST_FILE: &str = "project.json";
const ARCHIVE_DIR: &str = "_archive";

/// Portable project manifest (`project.json`, CR-02 §20). A project moved
/// to another machine is re-identified by this file plus `project.db`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectManifest {
    pub project_id: String,
    pub name: String,
    pub schema_version: u32,
    pub created_at: String,
}

/// An opened project: manifest + live store + root path.
pub struct OpenedProject {
    pub root: PathBuf,
    pub manifest: ProjectManifest,
    pub store: DomainStore,
}

/// Result of `recover_project` (CR-02 §28 substrate; interrupted-run
/// detection lands with pipeline persistence in CR-02.5).
#[derive(Debug, Clone)]
pub struct RecoveryReport {
    pub project_id: String,
    pub status: ProjectStatus,
    /// Layout directories that were missing and have been recreated.
    pub repaired_dirs: Vec<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum ProjectError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("store error: {0}")]
    Store(#[from] DomainStoreError),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("project not found: {0}")]
    NotFound(String),
    #[error("project already exists: {0}")]
    AlreadyExists(String),
    #[error("delete requires explicit confirmation (CR-02 §26)")]
    NotConfirmed,
    #[error("invalid project directory: {0}")]
    InvalidLayout(String),
}

type Result<T> = std::result::Result<T, ProjectError>;

pub struct ProjectManager {
    projects_root: PathBuf,
}

impl ProjectManager {
    pub fn new(projects_root: PathBuf) -> Self {
        Self { projects_root }
    }

    /// Create a new project: directory layout + migrated `project.db` +
    /// manifest. The domain store records the project row (status NEW) and
    /// the PROJECT_CREATED event.
    pub fn create_project(
        &self,
        name: &str,
        target_id: Option<&str>,
        application_version: &str,
    ) -> Result<OpenedProject> {
        let slug = slugify(name);
        let root = self.projects_root.join(&slug);
        if root.exists() {
            return Err(ProjectError::AlreadyExists(slug));
        }
        for d in LAYOUT_DIRS {
            std::fs::create_dir_all(root.join(d))?;
        }
        let store = DomainStore::new(&root.join(DB_FILE))?;
        let project_id = store.create_project(name, target_id, application_version)?;
        let project = store.get_project(&project_id)?;
        let manifest = ProjectManifest {
            project_id,
            name: name.to_string(),
            schema_version: project.schema_version,
            created_at: project.created_at,
        };
        write_manifest(&root, &manifest)?;
        Ok(OpenedProject {
            root,
            manifest,
            store,
        })
    }

    /// Open an existing project by slug. Migrations apply automatically on
    /// open (CR-02 §30: "Updating AstroForge Project…", never raw SQL at the
    /// user). The manifest is checked against the db identity.
    pub fn open_project(&self, slug: &str) -> Result<OpenedProject> {
        let root = self.projects_root.join(slug);
        if !root.is_dir() {
            return Err(ProjectError::NotFound(slug.to_string()));
        }
        let manifest = read_manifest(&root)?;
        let store = DomainStore::new(&root.join(DB_FILE))?;
        let project = store.get_project(&manifest.project_id)?;
        if project.name != manifest.name {
            return Err(ProjectError::InvalidLayout(format!(
                "manifest/db name mismatch: '{}' vs '{}'",
                manifest.name, project.name
            )));
        }
        Ok(OpenedProject {
            root,
            manifest,
            store,
        })
    }

    /// List projects (manifests) in the root, excluding the archive.
    pub fn list_projects(&self) -> Result<Vec<ProjectManifest>> {
        let mut out = Vec::new();
        if !self.projects_root.is_dir() {
            return Ok(out);
        }
        for entry in std::fs::read_dir(&self.projects_root)? {
            let entry = entry?;
            let path = entry.path();
            if !path.is_dir() || entry.file_name() == ARCHIVE_DIR {
                continue;
            }
            if path.join(MANIFEST_FILE).exists() {
                out.push(read_manifest(&path)?);
            }
        }
        out.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(out)
    }

    /// Rename updates the project record + manifest only (CR-02 §39:
    /// "renamed without affecting artifacts"). The directory name is set at
    /// creation and stays stable — artifact paths recorded in the db are
    /// rooted there, so moving the directory would invalidate them.
    pub fn rename_project(&self, slug: &str, new_name: &str) -> Result<()> {
        let mut opened = self.open_project(slug)?;
        opened
            .store
            .rename_project(&opened.manifest.project_id, new_name)?;
        opened.manifest.name = new_name.to_string();
        write_manifest(&opened.root, &opened.manifest)?;
        Ok(())
    }

    /// Archive moves the project directory under `_archive/`. The project is
    /// untouched otherwise and can be restored by moving it back.
    pub fn archive_project(&self, slug: &str) -> Result<PathBuf> {
        let root = self.projects_root.join(slug);
        if !root.is_dir() {
            return Err(ProjectError::NotFound(slug.to_string()));
        }
        let archive_root = self.projects_root.join(ARCHIVE_DIR);
        std::fs::create_dir_all(&archive_root)?;
        let dest = archive_root.join(slug);
        if dest.exists() {
            return Err(ProjectError::AlreadyExists(format!("{ARCHIVE_DIR}/{slug}")));
        }
        std::fs::rename(&root, &dest)?;
        Ok(dest)
    }

    /// Delete a project directory entirely. CR-02 §26: source data has the
    /// strongest protection, so deletion requires `confirm: true`; anything
    /// else is an error, not a silent no-op.
    pub fn delete_project(&self, slug: &str, confirm: bool) -> Result<()> {
        if !confirm {
            return Err(ProjectError::NotConfirmed);
        }
        let root = self.projects_root.join(slug);
        if !root.is_dir() {
            return Err(ProjectError::NotFound(slug.to_string()));
        }
        std::fs::remove_dir_all(&root)?;
        Ok(())
    }

    /// Recover a project after an unexpected termination (CR-02 §28):
    /// reopen, apply pending migrations, repair any missing layout
    /// directories, and report state. Interrupted pipeline-run detection is
    /// layered on top in CR-02.5 once pipeline_runs are persistent.
    pub fn recover_project(&self, slug: &str) -> Result<RecoveryReport> {
        let opened = self.open_project(slug)?;
        let mut repaired = Vec::new();
        for d in LAYOUT_DIRS {
            let dir = opened.root.join(d);
            if !dir.is_dir() {
                std::fs::create_dir_all(&dir)?;
                repaired.push(d.to_string());
            }
        }
        let project = opened.store.get_project(&opened.manifest.project_id)?;
        Ok(RecoveryReport {
            project_id: project.project_id,
            status: project.status,
            repaired_dirs: repaired,
        })
    }
}

// ─── Helpers ────────────────────────────────────────────────────────────────

/// Filesystem-safe project slug: "M42 — Orion Nebula" -> "m42-orion-nebula".
fn slugify(name: &str) -> String {
    let mut out = String::with_capacity(name.len());
    let mut last_dash = true; // strip leading separators
    for c in name.chars() {
        if c.is_ascii_alphanumeric() {
            out.push(c.to_ascii_lowercase());
            last_dash = false;
        } else if !last_dash {
            out.push('-');
            last_dash = true;
        }
    }
    let trimmed = out.trim_end_matches('-').to_string();
    if trimmed.is_empty() {
        "project".to_string()
    } else {
        trimmed
    }
}

fn write_manifest(root: &Path, manifest: &ProjectManifest) -> Result<()> {
    let json = serde_json::to_string_pretty(manifest)?;
    std::fs::write(root.join(MANIFEST_FILE), json)?;
    Ok(())
}

fn read_manifest(root: &Path) -> Result<ProjectManifest> {
    let path = root.join(MANIFEST_FILE);
    if !path.exists() {
        return Err(ProjectError::InvalidLayout(format!(
            "missing {MANIFEST_FILE} in {}",
            root.display()
        )));
    }
    let json = std::fs::read_to_string(path)?;
    Ok(serde_json::from_str(&json)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::ProjectEventKind;

    fn temp_manager(tag: &str) -> (PathBuf, ProjectManager) {
        let dir = std::env::temp_dir().join(format!(
            "astroforge-projects-{tag}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        (dir.clone(), ProjectManager::new(dir))
    }

    #[test]
    fn create_builds_layout_db_and_manifest() {
        let (root, mgr) = temp_manager("create");
        let opened = mgr
            .create_project("M42 — Orion Nebula", None, "0.1.0")
            .unwrap();
        assert_eq!(opened.root, root.join("m42-orion-nebula"));
        for d in LAYOUT_DIRS {
            assert!(opened.root.join(d).is_dir(), "missing dir {d}");
        }
        assert!(opened.root.join(DB_FILE).exists());
        assert!(opened.root.join(MANIFEST_FILE).exists());

        // Domain state: status NEW + PROJECT_CREATED event recorded.
        let p = opened
            .store
            .get_project(&opened.manifest.project_id)
            .unwrap();
        assert_eq!(p.status, ProjectStatus::New);
        let events = opened.store.list_events(&p.project_id).unwrap();
        assert_eq!(events[0].kind, ProjectEventKind::ProjectCreated);

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn create_rejects_duplicate_slug() {
        let (root, mgr) = temp_manager("dup");
        mgr.create_project("M42", None, "0.1.0").unwrap();
        assert!(matches!(
            mgr.create_project("m42", None, "0.1.0"),
            Err(ProjectError::AlreadyExists(_))
        ));
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn open_round_trips_and_validates_identity() {
        let (root, mgr) = temp_manager("open");
        let created = mgr.create_project("M31", None, "0.1.0").unwrap();
        let pid = created.manifest.project_id.clone();
        drop(created);

        let reopened = mgr.open_project("m31").unwrap();
        assert_eq!(reopened.manifest.project_id, pid);
        // CR-05 P4 slice 3 — migration v4 (preview_runs) bumped the
        // schema version; the assertion still proves the version is
        // recorded correctly across project reopen.
        // CR-06 P1 — schema_version() bumped to 5 by the AI
        // Enhancement Studio migration (ai_operations,
        // image_analyses, image_regions, ai_recommendations,
        // ai_masks, enhancement_stacks, enhancement_previews).
        // CR-06 P4 — schema_version() bumped to 6 by the
        // image_versions migration.
        // CR-04 P8 — schema_version() bumped to 7 by the
        // session-classification ALTER TABLE migration.
        // CR-04 P10 — schema_version() bumped to 8 by the
        // target_provenance ALTER TABLE migration.
        assert_eq!(reopened.store.schema_version(), 8);

        // Tampered manifest is rejected.
        let mut bad = reopened.manifest.clone();
        bad.name = "Tampered".into();
        write_manifest(&reopened.root, &bad).unwrap();
        assert!(matches!(
            mgr.open_project("m31"),
            Err(ProjectError::InvalidLayout(_))
        ));
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn rename_updates_metadata_without_touching_layout() {
        let (root, mgr) = temp_manager("rename");
        let opened = mgr.create_project("M42", None, "0.1.0").unwrap();
        let pid = opened.manifest.project_id.clone();
        let dir = opened.root.clone();
        drop(opened);

        mgr.rename_project("m42", "M42 — Orion Nebula").unwrap();
        let reopened = mgr.open_project("m42").unwrap();
        assert_eq!(reopened.manifest.name, "M42 — Orion Nebula");
        assert_eq!(
            reopened.store.get_project(&pid).unwrap().name,
            "M42 — Orion Nebula"
        );
        // Directory name stable → artifact paths stay valid.
        assert_eq!(reopened.root, dir);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn archive_moves_and_unlists_project() {
        let (root, mgr) = temp_manager("archive");
        mgr.create_project("Horsehead", None, "0.1.0").unwrap();
        let dest = mgr.archive_project("horsehead").unwrap();
        assert!(dest.is_dir());
        assert!(mgr.list_projects().unwrap().is_empty());
        assert!(matches!(
            mgr.open_project("horsehead"),
            Err(ProjectError::NotFound(_))
        ));
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn delete_requires_explicit_confirmation() {
        let (root, mgr) = temp_manager("delete");
        mgr.create_project("M45", None, "0.1.0").unwrap();
        assert!(matches!(
            mgr.delete_project("m45", false),
            Err(ProjectError::NotConfirmed)
        ));
        assert!(root.join("m45").is_dir());
        mgr.delete_project("m45", true).unwrap();
        assert!(!root.join("m45").exists());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn recover_repairs_missing_layout_dirs() {
        let (root, mgr) = temp_manager("recover");
        let opened = mgr.create_project("IC434", None, "0.1.0").unwrap();
        let pid = opened.manifest.project_id.clone();
        let proj_root = opened.root.clone();
        drop(opened);

        // Simulate damage: two layout directories lost.
        std::fs::remove_dir_all(proj_root.join("artifacts")).unwrap();
        std::fs::remove_dir_all(proj_root.join("previews")).unwrap();

        let report = mgr.recover_project("ic434").unwrap();
        assert_eq!(report.project_id, pid);
        assert_eq!(report.status, ProjectStatus::New);
        assert_eq!(report.repaired_dirs.len(), 2);
        assert!(proj_root.join("artifacts").is_dir());
        assert!(proj_root.join("previews").is_dir());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn slugify_handles_unicode_and_empty() {
        assert_eq!(slugify("M42 — Orion Nebula"), "m42-orion-nebula");
        assert_eq!(slugify("  NGC 7000  "), "ngc-7000");
        assert_eq!(slugify("———"), "project");
    }
}
