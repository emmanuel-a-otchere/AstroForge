//! Local recipe (pipeline profile) store backed by rusqlite.
//!
//! Schema lives in `db::RECIPE_SCHEMA_SQL`. Each row is one version of
//! a profile; the `profile_id` groups versions of the same logical
//! profile. The active head of each profile is the row with the highest
//! `version` for that `profile_id`.
//!
//! D-1 = 2 (linear history): every save increments `version`; users see
//! the active head plus full version history per profile.
//!
//! D-3 = 1 (soft migration): on read, the `payload_json` is deserialized
//! via `Recipe::from_json_migrated()` so older schemas upgrade
//! transparently and the migrated form gets re-persisted on next save.

use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Mutex;

use crate::db;
use crate::recipe::{migrate_recipe, Recipe};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RecipeSummary {
    pub id: i64,
    pub profile_id: String,
    pub schema_version: String,
    pub name: String,
    pub description: String,
    pub target_type: String,
    pub version: u32,
    pub parent_version: Option<u32>,
    pub branch: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecipeVersion {
    pub version: u32,
    pub parent_version: Option<u32>,
    pub branch: String,
    pub created_at: String,
}

#[derive(Debug, thiserror::Error)]
pub enum RecipeStoreError {
    #[error("sqlite error: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("recipe not found: profile_id={profile_id}, version={version}")]
    NotFound { profile_id: String, version: u32 },
}

pub struct RecipeStore {
    conn: Mutex<Connection>,
}

fn row_to_summary(row: &rusqlite::Row<'_>) -> rusqlite::Result<RecipeSummary> {
    Ok(RecipeSummary {
        id: row.get(0)?,
        profile_id: row.get(1)?,
        schema_version: row.get(2)?,
        name: row.get(3)?,
        description: row.get(4)?,
        target_type: row.get(5)?,
        version: row.get::<_, i64>(6)? as u32,
        parent_version: row.get::<_, Option<i64>>(7)?.map(|v| v as u32),
        branch: row.get(8)?,
        created_at: row.get(9)?,
    })
}

impl RecipeStore {
    pub fn new(db_path: &PathBuf) -> Result<Self, RecipeStoreError> {
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let conn = Connection::open(db_path)?;
        conn.execute_batch(db::RECIPE_SCHEMA_SQL)?;
        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    /// Compute the deterministic `profile_id` for a recipe. Two recipes
    /// with the same name + target_type share the same profile_id and
    /// therefore share a version history.
    pub fn profile_id_for(name: &str, target_type: &str) -> String {
        // Format: `prof_<sanitized-name>_<sanitized-target-type>`. We keep
        // it human-readable for debugging rather than hashing.
        let sanitized = |s: &str| -> String {
            s.chars()
                .map(|c| {
                    if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                        c.to_ascii_lowercase()
                    } else {
                        '-'
                    }
                })
                .collect::<String>()
                .trim_matches('-')
                .to_string()
        };
        format!("prof_{}_{}", sanitized(name), sanitized(target_type))
    }

    /// Compute the next version number for a profile. Returns 1 if the
    /// profile is new.
    pub fn next_version_for(&self, profile_id: &str) -> Result<u32, RecipeStoreError> {
        let conn = self.conn.lock().expect("recipe db mutex poisoned");
        let max: Option<Option<i64>> = conn
            .query_row(
                "SELECT MAX(version) FROM recipes WHERE profile_id = ?1",
                params![profile_id],
                |row| row.get::<_, Option<i64>>(0),
            )
            .optional()?;
        // Outer Option = row presence; inner Option = NULL inside MAX().
        Ok(max.flatten().map(|v| (v as u32) + 1).unwrap_or(1))
    }

    /// Save a new version of a recipe. Caller is responsible for setting
    /// `version` correctly — use `next_version_for()` to compute it
    /// before calling.
    pub fn save(&self, recipe: &Recipe) -> Result<RecipeSummary, RecipeStoreError> {
        // Defensive: ensure migration to current before persisting.
        let mut owned = recipe.clone();
        migrate_recipe(&mut owned);
        let payload = owned.to_json()?;

        let profile_id = Self::profile_id_for(&owned.name, &owned.target_type);
        let conn = self.conn.lock().expect("recipe db mutex poisoned");
        conn.execute(
            "INSERT INTO recipes \
                (profile_id, schema_version, name, description, target_type, \
                 version, parent_version, branch, created_at, payload_json) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, datetime('now'), ?9)",
            params![
                profile_id,
                owned.schema_version,
                owned.name,
                owned.description,
                owned.target_type,
                owned.version as i64,
                owned.parent_version.map(|v| v as i64),
                owned.branch,
                payload,
            ],
        )?;

        let mut stmt = conn.prepare(
            "SELECT id, profile_id, schema_version, name, description, target_type, \
                    version, parent_version, branch, created_at \
             FROM recipes WHERE profile_id = ?1 AND version = ?2 AND branch = ?3",
        )?;
        let row = stmt.query_row(
            params![profile_id, owned.version as i64, owned.branch],
            row_to_summary,
        )?;
        Ok(row)
    }

    /// List the active head of every profile, newest-first by created_at.
    pub fn list(&self) -> Result<Vec<RecipeSummary>, RecipeStoreError> {
        let conn = self.conn.lock().expect("recipe db mutex poisoned");
        // Active head per profile_id = row with MAX(version) for that id.
        let mut stmt = conn.prepare(
            "SELECT r.id, r.profile_id, r.schema_version, r.name, r.description, \
                    r.target_type, r.version, r.parent_version, r.branch, r.created_at \
             FROM recipes r \
             INNER JOIN ( \
                 SELECT profile_id, MAX(version) AS max_version \
                 FROM recipes WHERE branch = 'main' \
                 GROUP BY profile_id \
             ) latest ON r.profile_id = latest.profile_id \
                      AND r.version = latest.max_version \
                      AND r.branch = 'main' \
             ORDER BY r.created_at DESC, r.name ASC",
        )?;
        let rows = stmt
            .query_map([], row_to_summary)?
            .collect::<Result<Vec<_>, rusqlite::Error>>()?;
        Ok(rows)
    }

    /// List every version of a single profile (across the main branch),
    /// newest-first.
    pub fn list_versions(&self, profile_id: &str) -> Result<Vec<RecipeVersion>, RecipeStoreError> {
        let conn = self.conn.lock().expect("recipe db mutex poisoned");
        let mut stmt = conn.prepare(
            "SELECT version, parent_version, branch, created_at \
             FROM recipes WHERE profile_id = ?1 AND branch = 'main' \
             ORDER BY version DESC",
        )?;
        let rows = stmt
            .query_map(params![profile_id], |row| {
                Ok(RecipeVersion {
                    version: row.get::<_, i64>(0)? as u32,
                    parent_version: row.get::<_, Option<i64>>(1)?.map(|v| v as u32),
                    branch: row.get(2)?,
                    created_at: row.get(3)?,
                })
            })?
            .collect::<Result<Vec<_>, rusqlite::Error>>()?;
        Ok(rows)
    }

    /// Load the full Recipe for a given profile_id + version. Migrates
    /// the on-disk payload to current schema before returning.
    pub fn get(&self, profile_id: &str, version: u32) -> Result<Recipe, RecipeStoreError> {
        let conn = self.conn.lock().expect("recipe db mutex poisoned");
        let payload: String = conn
            .query_row(
                "SELECT payload_json FROM recipes \
                 WHERE profile_id = ?1 AND version = ?2 AND branch = 'main'",
                params![profile_id, version as i64],
                |row| row.get(0),
            )
            .map_err(|e| match e {
                rusqlite::Error::QueryReturnedNoRows => RecipeStoreError::NotFound {
                    profile_id: profile_id.to_string(),
                    version,
                },
                other => RecipeStoreError::Sqlite(other),
            })?;
        let recipe = Recipe::from_json_migrated(&payload)?;
        Ok(recipe)
    }

    /// Load the active head of a profile (highest version on main).
    pub fn get_head(&self, profile_id: &str) -> Result<Recipe, RecipeStoreError> {
        let conn = self.conn.lock().expect("recipe db mutex poisoned");
        let payload: String = conn
            .query_row(
                "SELECT payload_json FROM recipes \
                 WHERE profile_id = ?1 AND branch = 'main' \
                 ORDER BY version DESC LIMIT 1",
                params![profile_id],
                |row| row.get(0),
            )
            .map_err(|e| match e {
                rusqlite::Error::QueryReturnedNoRows => RecipeStoreError::NotFound {
                    profile_id: profile_id.to_string(),
                    version: 0,
                },
                other => RecipeStoreError::Sqlite(other),
            })?;
        let recipe = Recipe::from_json_migrated(&payload)?;
        Ok(recipe)
    }

    /// Seed the DwarfII v1 profile if no profile with the same
    /// `profile_id` exists yet. Idempotent.
    pub fn seed_if_empty(&self) -> Result<(), RecipeStoreError> {
        let dwarf_id = Self::profile_id_for(
            crate::seed::DWARF2_V1_NAME,
            crate::seed::DWARF2_V1_TARGET_TYPE,
        );
        let conn = self.conn.lock().expect("recipe db mutex poisoned");
        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM recipes WHERE profile_id = ?1",
            params![dwarf_id],
            |row| row.get(0),
        )?;
        if count > 0 {
            return Ok(());
        }
        drop(conn);
        let recipe = crate::seed::dwarf2_v1();
        self.save(&recipe)?;
        Ok(())
    }
}

// `optional()` is a method on rusqlite's Result that converts
// QueryReturnedNoRows into Ok(None). Bring it into scope locally.
use rusqlite::OptionalExtension as _;

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn temp_db() -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "astroforge_recipe_test_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        dir.join("recipes.sqlite")
    }

    fn sample(name: &str) -> Recipe {
        let mut r = Recipe::new(name, "smart_telescope_osc");
        r.add_stage("denoise", Default::default());
        r
    }

    #[test]
    fn test_save_and_get_roundtrip() {
        let store = RecipeStore::new(&temp_db()).unwrap();
        let r = sample("DwarfII");
        let summary = store.save(&r).unwrap();
        assert_eq!(summary.version, 1);
        assert_eq!(summary.name, "DwarfII");

        let loaded = store.get(&summary.profile_id, 1).unwrap();
        assert_eq!(loaded.name, "DwarfII");
        assert_eq!(loaded.stages.len(), 1);
    }

    #[test]
    fn test_profile_id_is_deterministic_for_name_target() {
        assert_eq!(
            RecipeStore::profile_id_for("DwarfII", "smart_telescope_osc"),
            RecipeStore::profile_id_for("DwarfII", "smart_telescope_osc"),
        );
        assert_ne!(
            RecipeStore::profile_id_for("DwarfII", "smart_telescope_osc"),
            RecipeStore::profile_id_for("DwarfII", "deep_sky"),
        );
    }

    #[test]
    fn test_profile_id_sanitizes_special_chars() {
        let id = RecipeStore::profile_id_for("Dwarf II Smart!", "OSC");
        // Spaces and ! become '-'; everything lowercased.
        assert!(id.starts_with("prof_"));
        assert!(id.contains("dwarf-ii-smart"));
        assert!(id.contains("osc"));
    }

    #[test]
    fn test_next_version_starts_at_one_then_increments() {
        let store = RecipeStore::new(&temp_db()).unwrap();
        let id = RecipeStore::profile_id_for("DwarfII", "smart_telescope_osc");
        assert_eq!(store.next_version_for(&id).unwrap(), 1);

        let mut r1 = sample("DwarfII");
        r1.version = store.next_version_for(&id).unwrap();
        store.save(&r1).unwrap();
        assert_eq!(store.next_version_for(&id).unwrap(), 2);

        let mut r2 = sample("DwarfII");
        r2.version = store.next_version_for(&id).unwrap();
        r2.parent_version = Some(1);
        store.save(&r2).unwrap();
        assert_eq!(store.next_version_for(&id).unwrap(), 3);
    }

    #[test]
    fn test_list_returns_head_per_profile() {
        let store = RecipeStore::new(&temp_db()).unwrap();
        let mut r1 = sample("DwarfII");
        r1.version = 1;
        store.save(&r1).unwrap();
        let mut r2 = sample("DwarfII");
        r2.version = 2;
        store.save(&r2).unwrap();
        let mut r3 = sample("GenericOSC");
        r3.version = 1;
        store.save(&r3).unwrap();

        let list = store.list().unwrap();
        assert_eq!(list.len(), 2);

        let dwarfii = list.iter().find(|s| s.name == "DwarfII").unwrap();
        assert_eq!(dwarfii.version, 2, "head should be the max version");

        let generic = list.iter().find(|s| s.name == "GenericOSC").unwrap();
        assert_eq!(generic.version, 1);
    }

    #[test]
    fn test_list_versions_descending() {
        let store = RecipeStore::new(&temp_db()).unwrap();
        for v in 1..=3 {
            let mut r = sample("DwarfII");
            r.version = v;
            r.parent_version = if v == 1 { None } else { Some(v - 1) };
            store.save(&r).unwrap();
        }
        let id = RecipeStore::profile_id_for("DwarfII", "smart_telescope_osc");
        let versions = store.list_versions(&id).unwrap();
        assert_eq!(versions.len(), 3);
        assert_eq!(versions[0].version, 3);
        assert_eq!(versions[0].parent_version, Some(2));
        assert_eq!(versions[2].version, 1);
        assert_eq!(versions[2].parent_version, None);
    }

    #[test]
    fn test_get_head_returns_highest_version() {
        let store = RecipeStore::new(&temp_db()).unwrap();
        for v in 1..=2 {
            let mut r = sample("DwarfII");
            r.version = v;
            store.save(&r).unwrap();
        }
        let id = RecipeStore::profile_id_for("DwarfII", "smart_telescope_osc");
        let head = store.get_head(&id).unwrap();
        assert_eq!(head.version, 2);
    }

    #[test]
    fn test_save_migrates_v1_payload_before_persisting() {
        let store = RecipeStore::new(&temp_db()).unwrap();
        // Build a v1-shaped Recipe (set schema_version explicitly).
        let mut r = sample("DwarfII");
        r.schema_version = crate::recipe::SCHEMA_VERSION_V1.into();
        // `Recipe::new` already populates the new fields with defaults,
        // but the schema_version is what triggers migration in save().
        let summary = store.save(&r).unwrap();
        assert_eq!(
            summary.schema_version,
            crate::recipe::SCHEMA_VERSION_CURRENT
        );

        let loaded = store.get(&summary.profile_id, 1).unwrap();
        assert_eq!(loaded.schema_version, crate::recipe::SCHEMA_VERSION_CURRENT);
        assert_eq!(loaded.version, 1);
    }

    #[test]
    fn test_seed_dwarf2_is_idempotent() {
        let store = RecipeStore::new(&temp_db()).unwrap();
        store.seed_if_empty().unwrap();
        let list1 = store.list().unwrap();
        assert_eq!(list1.len(), 1);
        assert_eq!(list1[0].name, crate::seed::DWARF2_V1_NAME);
        assert_eq!(list1[0].version, 1);

        // Calling again does not duplicate.
        store.seed_if_empty().unwrap();
        let list2 = store.list().unwrap();
        assert_eq!(list2.len(), 1);
    }

    #[test]
    fn test_not_found_for_missing_version() {
        let store = RecipeStore::new(&temp_db()).unwrap();
        let id = RecipeStore::profile_id_for("DwarfII", "smart_telescope_osc");
        let result = store.get(&id, 1);
        assert!(matches!(result, Err(RecipeStoreError::NotFound { .. })));
    }
}
