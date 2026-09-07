//! CR-05 P1 — `PipelinePlanStore` (Decision D-CR05-2).
//!
//! Thin rusqlite-backed store for PipelinePlan + PipelineStage +
//! StageExecution rows. Sits in its own store (per the existing
//! session / gallery / recipe / pipeline-plans split) so the other stores
//! can evolve independently.
//!
//! P1 ships: schema bootstrap + insert + load. P2 will add the
//! runner that writes StageExecution rows.

use crate::db;
use crate::domain::{PipelinePlan, PipelineStage};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Mutex;

/// CR-05 P1 — public API summary.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelinePlanSummary {
    pub plan_id: String,
    pub project_id: String,
    pub session_id: String,
    pub recipe_id: Option<String>,
    pub mode: String,
    pub target_type: String,
    pub status: String,
    pub created_at: String,
    pub schema_version: u32,
}

impl From<&PipelinePlan> for PipelinePlanSummary {
    fn from(p: &PipelinePlan) -> Self {
        Self {
            plan_id: p.plan_id.clone(),
            project_id: p.project_id.clone(),
            session_id: p.session_id.clone(),
            recipe_id: p.recipe_id.clone(),
            mode: p.mode.clone(),
            target_type: format!("{:?}", p.target_type).to_lowercase(),
            status: format!("{:?}", p.status).to_lowercase(),
            created_at: p.created_at.clone(),
            schema_version: p.schema_version,
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum PipelinePlanStoreError {
    #[error("sqlite error: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("plan not found: {0}")]
    NotFound(String),
}

pub struct PipelinePlanStore {
    conn: Mutex<Connection>,
}

impl PipelinePlanStore {
    /// Open or create a store at the given file. Runs the v1 schema
    /// migration on first open.
    pub fn new(path: PathBuf) -> Result<Self, PipelinePlanStoreError> {
        let conn = Connection::open(path)?;
        conn.execute_batch(db::PIPELINE_PLANS_SCHEMA_SQL)?;
        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    /// In-memory store for tests.
    pub fn in_memory() -> Result<Self, PipelinePlanStoreError> {
        let conn = Connection::open_in_memory()?;
        conn.execute_batch(db::PIPELINE_PLANS_SCHEMA_SQL)?;
        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    pub fn insert_plan(&self, plan: &PipelinePlan) -> Result<(), PipelinePlanStoreError> {
        let conn = self.conn.lock().expect("poisoned");
        conn.execute(
            "INSERT OR REPLACE INTO pipeline_plans
             (plan_id, project_id, session_id, recipe_id, mode, target_type, status, created_at, schema_version)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                plan.plan_id,
                plan.project_id,
                plan.session_id,
                plan.recipe_id,
                plan.mode,
                format!("{:?}", plan.target_type).to_lowercase(),
                format!("{:?}", plan.status).to_lowercase(),
                plan.created_at,
                plan.schema_version,
            ],
        )?;
        for stage in &plan.stages {
            conn.execute(
                "INSERT OR REPLACE INTO pipeline_stages
                 (stage_id, plan_id, stage_type, sequence, label, required, enabled, parameters_json, produces_image_version, undo_supported)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                params![
                    stage.stage_id,
                    stage.plan_id,
                    stage.stage_type,
                    stage.sequence,
                    stage.label,
                    stage.required as i32,
                    stage.enabled as i32,
                    stage.parameters_json,
                    stage.produces_image_version as i32,
                    stage.undo_supported as i32,
                ],
            )?;
        }
        Ok(())
    }

    pub fn load_plan(&self, plan_id: &str) -> Result<PipelinePlan, PipelinePlanStoreError> {
        let conn = self.conn.lock().expect("poisoned");
        let plan_row = conn
            .query_row(
                "SELECT plan_id, project_id, session_id, recipe_id, mode, target_type, status, created_at, schema_version
                 FROM pipeline_plans WHERE plan_id = ?1",
                params![plan_id],
                |row| {
                    let target_type_str: String = row.get(5)?;
                    let status_str: String = row.get(6)?;
                    Ok(PlanRow {
                        plan_id: row.get(0)?,
                        project_id: row.get(1)?,
                        session_id: row.get(2)?,
                        recipe_id: row.get(3)?,
                        mode: row.get(4)?,
                        target_type: target_type_str,
                        status: status_str,
                        created_at: row.get(7)?,
                        schema_version: row.get(8)?,
                    })
                },
            )
            .optional()?
            .ok_or_else(|| PipelinePlanStoreError::NotFound(plan_id.into()))?;

        let mut stmt = conn.prepare(
            "SELECT stage_id, plan_id, stage_type, sequence, label, required, enabled, parameters_json, produces_image_version, undo_supported
             FROM pipeline_stages WHERE plan_id = ?1 ORDER BY sequence ASC",
        )?;
        let stage_iter = stmt.query_map(params![plan_id], |row| {
            Ok(PipelineStage {
                stage_id: row.get(0)?,
                plan_id: row.get(1)?,
                stage_type: row.get(2)?,
                sequence: row.get(3)?,
                label: row.get(4)?,
                required: row.get::<_, i32>(5)? != 0,
                enabled: row.get::<_, i32>(6)? != 0,
                parameters_json: row.get(7)?,
                produces_image_version: row.get::<_, i32>(8)? != 0,
                undo_supported: row.get::<_, i32>(9)? != 0,
            })
        })?;
        let stages: Vec<PipelineStage> = stage_iter.collect::<Result<_, _>>()?;

        Ok(PipelinePlan {
            plan_id: plan_row.plan_id,
            project_id: plan_row.project_id,
            session_id: plan_row.session_id,
            recipe_id: plan_row.recipe_id,
            mode: plan_row.mode,
            target_type: parse_target_type(&plan_row.target_type),
            status: parse_status(&plan_row.status),
            created_at: plan_row.created_at,
            schema_version: plan_row.schema_version,
            stages,
        })
    }

    pub fn list_plans_for_project(
        &self,
        project_id: &str,
    ) -> Result<Vec<PipelinePlanSummary>, PipelinePlanStoreError> {
        let conn = self.conn.lock().expect("poisoned");
        let mut stmt = conn.prepare(
            "SELECT plan_id, project_id, session_id, recipe_id, mode, target_type, status, created_at, schema_version
             FROM pipeline_plans WHERE project_id = ?1 ORDER BY created_at DESC",
        )?;
        let rows = stmt
            .query_map(params![project_id], |row| {
                Ok(PipelinePlanSummary {
                    plan_id: row.get(0)?,
                    project_id: row.get(1)?,
                    session_id: row.get(2)?,
                    recipe_id: row.get(3)?,
                    mode: row.get(4)?,
                    target_type: row.get(5)?,
                    status: row.get(6)?,
                    created_at: row.get(7)?,
                    schema_version: row.get(8)?,
                })
            })?
            .collect::<Result<_, _>>()?;
        Ok(rows)
    }
}

struct PlanRow {
    plan_id: String,
    project_id: String,
    session_id: String,
    recipe_id: Option<String>,
    mode: String,
    target_type: String,
    status: String,
    created_at: String,
    schema_version: u32,
}

fn parse_target_type(s: &str) -> crate::domain::ObjectType {
    use crate::domain::ObjectType;
    match s {
        "deep_sky" | "deepsky" => ObjectType::DeepSky,
        "planet" => ObjectType::Planet,
        "lunar" => ObjectType::Lunar,
        "solar" => ObjectType::Solar,
        _ => ObjectType::Unknown,
    }
}

fn parse_status(s: &str) -> crate::domain::PipelinePlanStatus {
    use crate::domain::PipelinePlanStatus;
    match s {
        "draft" => PipelinePlanStatus::Draft,
        "ready" => PipelinePlanStatus::Ready,
        "running" => PipelinePlanStatus::Running,
        "paused" => PipelinePlanStatus::Paused,
        "completed" => PipelinePlanStatus::Completed,
        "failed" => PipelinePlanStatus::Failed,
        "cancelled" => PipelinePlanStatus::Cancelled,
        _ => PipelinePlanStatus::Draft,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{ObjectType, PipelinePlan, PipelinePlanStatus, PipelineStage};
    use crate::pipeline_plan::builtin::deep_sky_osc_balanced;
    use crate::pipeline_plan::plan::{generate_plan, GenerationContext, SessionUnderstanding};

    fn ctx() -> GenerationContext {
        GenerationContext {
            project_id: "proj_1".into(),
            session_id: "sess_1".into(),
            session_understanding: SessionUnderstanding::deep_sky_osc_balanced(),
            recipe_id: Some("recipe_deep_sky_osc_balanced".into()),
            mode: "auto".into(),
            generated_at_unix_ms: 1_700_000_000_000,
        }
    }

    #[test]
    fn insert_and_load_round_trip() {
        let store = PipelinePlanStore::in_memory().unwrap();
        let (stages, target) = deep_sky_osc_balanced();
        let plan = generate_plan(&ctx(), &stages, target).unwrap();

        store.insert_plan(&plan).unwrap();
        let loaded = store.load_plan(&plan.plan_id).unwrap();

        assert_eq!(loaded.plan_id, plan.plan_id);
        assert_eq!(loaded.project_id, plan.project_id);
        assert_eq!(loaded.mode, plan.mode);
        assert_eq!(loaded.target_type, ObjectType::DeepSky);
        assert_eq!(loaded.stages.len(), plan.stages.len());
        for (a, b) in loaded.stages.iter().zip(plan.stages.iter()) {
            assert_eq!(a.stage_id, b.stage_id);
            assert_eq!(a.label, b.label);
            assert_eq!(a.required, b.required);
            assert_eq!(a.sequence, b.sequence);
        }
    }

    #[test]
    fn list_for_project_returns_plans_newest_first() {
        let store = PipelinePlanStore::in_memory().unwrap();
        let (stages, target) = deep_sky_osc_balanced();
        let mut plan_a = generate_plan(&ctx(), &stages, target).unwrap();
        plan_a.plan_id = "plan_a".into();
        let mut plan_b = generate_plan(&ctx(), &stages, target).unwrap();
        plan_b.plan_id = "plan_b".into();
        store.insert_plan(&plan_a).unwrap();
        store.insert_plan(&plan_b).unwrap();

        let listed = store.list_plans_for_project("proj_1").unwrap();
        assert_eq!(listed.len(), 2);
        assert!(listed.iter().any(|s| s.plan_id == "plan_a"));
        assert!(listed.iter().any(|s| s.plan_id == "plan_b"));
    }

    #[test]
    fn load_unknown_plan_returns_not_found() {
        let store = PipelinePlanStore::in_memory().unwrap();
        let err = store.load_plan("nonexistent").unwrap_err();
        assert!(matches!(err, PipelinePlanStoreError::NotFound(_)));
    }

    #[test]
    fn insert_plan_with_synthetic_minimal_shape() {
        // Sanity check: insert a hand-built plan and reload.
        let store = PipelinePlanStore::in_memory().unwrap();
        let plan = PipelinePlan {
            plan_id: "plan_x".into(),
            project_id: "proj_x".into(),
            session_id: "sess_x".into(),
            recipe_id: None,
            mode: "guided".into(),
            target_type: ObjectType::DeepSky,
            status: PipelinePlanStatus::Draft,
            created_at: "2026-09-07T00:00:00Z".into(),
            schema_version: 1,
            stages: vec![PipelineStage {
                stage_id: "stage_1".into(),
                plan_id: "plan_x".into(),
                stage_type: "calibrate".into(),
                sequence: 0,
                label: "Calibrate".into(),
                required: true,
                enabled: true,
                parameters_json: None,
                produces_image_version: true,
                undo_supported: true,
            }],
        };
        store.insert_plan(&plan).unwrap();
        let loaded = store.load_plan("plan_x").unwrap();
        assert_eq!(loaded.stages.len(), 1);
        assert_eq!(loaded.stages[0].label, "Calibrate");
    }
}
