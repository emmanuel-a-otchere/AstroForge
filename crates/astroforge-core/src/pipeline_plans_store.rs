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
use crate::domain::{PipelinePlan, PipelineStage, StageExecution};
use crate::recommendation::Recommendation;
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
    /// CR-05 P3 slice 2 — JSON serialisation failure inside the
    /// store (e.g. `decision_json` for a Recommendation row).
    #[error("json serialisation error: {0}")]
    Json(#[from] serde_json::Error),
}

pub struct PipelinePlanStore {
    conn: Mutex<Connection>,
}

impl PipelinePlanStore {
    /// Open or create a store at the given file. Runs the schema
    /// migrations on first open.
    pub fn new(path: PathBuf) -> Result<Self, PipelinePlanStoreError> {
        let conn = Connection::open(path)?;
        conn.execute_batch(db::PIPELINE_PLANS_SCHEMA_SQL)?;
        Self::run_migrations(&conn)?;
        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    /// In-memory store for tests.
    pub fn in_memory() -> Result<Self, PipelinePlanStoreError> {
        let conn = Connection::open_in_memory()?;
        conn.execute_batch(db::PIPELINE_PLANS_SCHEMA_SQL)?;
        Self::run_migrations(&conn)?;
        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    /// CR-05 P3 slice 1 + slice 2.5 — apply schema migrations.
    /// SQLite does not support `ADD COLUMN IF NOT EXISTS`, so each
    /// migration is gated by a `pragma_table_info` check that
    /// returns true once the column exists. Idempotent across
    /// re-opens.
    fn run_migrations(conn: &Connection) -> Result<(), PipelinePlanStoreError> {
        // v2: add metric_snapshot_json to stage_executions.
        Self::add_column_if_missing(
            conn,
            "stage_executions",
            "metric_snapshot_json",
            "ALTER TABLE stage_executions ADD COLUMN metric_snapshot_json TEXT",
        )?;
        // v3: P3 slice 2.5 — user decision persistence on recommendations.
        Self::add_column_if_missing(
            conn,
            "recommendations",
            "user_decision",
            "ALTER TABLE recommendations ADD COLUMN user_decision TEXT",
        )?;
        Self::add_column_if_missing(
            conn,
            "recommendations",
            "user_decision_at",
            "ALTER TABLE recommendations ADD COLUMN user_decision_at TEXT",
        )?;
        Self::add_column_if_missing(
            conn,
            "recommendations",
            "applied_stage_id",
            "ALTER TABLE recommendations ADD COLUMN applied_stage_id TEXT",
        )?;
        // v4: P4 slice 6 — preview-before-apply provenance.
        // Records the preview_run.id that satisfied the §11 gate
        // when the recommendation was applied.
        Self::add_column_if_missing(
            conn,
            "recommendations",
            "applied_with_preview_id",
            "ALTER TABLE recommendations ADD COLUMN applied_with_preview_id TEXT",
        )?;
        Ok(())
    }

    /// CR-05 P3 slice 2.5 — add a column if `pragma_table_info`
    /// doesn't already list it. `pragma_table_info` is a virtual
    /// table so the column-name lookup is a regular `SELECT`.
    fn add_column_if_missing(
        conn: &Connection,
        table: &str,
        column: &str,
        alter_sql: &str,
    ) -> Result<(), PipelinePlanStoreError> {
        let mut stmt = conn.prepare(&format!(
            "SELECT 1 FROM pragma_table_info('{}') WHERE name = ?1",
            table
        ))?;
        let mut rows = stmt.query([column])?;
        let present = rows.next()?.is_some();
        drop(rows);
        drop(stmt);
        if !present {
            conn.execute_batch(alter_sql)?;
        }
        Ok(())
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

    // ─── Stage execution CRUD (CR-05 P2 slice 1) ───────────────────────
    //
    // P2 ships start + cancel only; pause / resume land in P2.5. The
    // runner calls `insert_stage_execution` when each stage begins and
    // `update_stage_execution` when it completes / fails / is cancelled.

    pub fn insert_stage_execution(
        &self,
        exec: &StageExecution,
    ) -> Result<(), PipelinePlanStoreError> {
        let conn = self.conn.lock().expect("poisoned");
        conn.execute(
            "INSERT OR REPLACE INTO stage_executions
             (stage_execution_id, plan_id, stage_id, attempt, status, input_version_id, output_artifact_id, parameters_json, parameters_hash, started_at, completed_at, resource_usage_json, error_json, metric_snapshot_json)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
            params![
                exec.stage_execution_id,
                exec.plan_id,
                exec.stage_id,
                exec.attempt,
                exec.status,
                exec.input_version_id,
                exec.output_artifact_id,
                exec.parameters_json,
                exec.parameters_hash,
                exec.started_at,
                exec.completed_at,
                exec.resource_usage_json,
                exec.error_json,
                exec.metric_snapshot_json,
            ],
        )?;
        Ok(())
    }

    pub fn update_stage_execution(
        &self,
        exec: &StageExecution,
    ) -> Result<(), PipelinePlanStoreError> {
        // Same SQL as insert (PK collision triggers OR REPLACE).
        self.insert_stage_execution(exec)
    }

    pub fn list_stage_executions_for_plan(
        &self,
        plan_id: &str,
    ) -> Result<Vec<StageExecution>, PipelinePlanStoreError> {
        let conn = self.conn.lock().expect("poisoned");
        let mut stmt = conn.prepare(
            "SELECT stage_execution_id, plan_id, stage_id, attempt, status, input_version_id, output_artifact_id, parameters_json, parameters_hash, started_at, completed_at, resource_usage_json, error_json, metric_snapshot_json
             FROM stage_executions WHERE plan_id = ?1 ORDER BY started_at ASC",
        )?;
        let rows = stmt
            .query_map(params![plan_id], |row| {
                Ok(StageExecution {
                    stage_execution_id: row.get(0)?,
                    plan_id: row.get(1)?,
                    stage_id: row.get(2)?,
                    attempt: row.get(3)?,
                    status: row.get(4)?,
                    input_version_id: row.get(5)?,
                    output_artifact_id: row.get(6)?,
                    parameters_json: row.get(7)?,
                    parameters_hash: row.get(8)?,
                    started_at: row.get(9)?,
                    completed_at: row.get(10)?,
                    resource_usage_json: row.get(11)?,
                    error_json: row.get(12)?,
                    metric_snapshot_json: row.get(13)?,
                })
            })?
            .collect::<Result<_, _>>()?;
        Ok(rows)
    }

    /// CR-05 P2 slice 1 — update the plan's status (Draft / Ready /
    /// Running / Paused / Completed / Failed / Cancelled). The P2.5
    /// pause/resume path will use this too.
    pub fn update_plan_status(
        &self,
        plan_id: &str,
        status: crate::domain::PipelinePlanStatus,
    ) -> Result<(), PipelinePlanStoreError> {
        let conn = self.conn.lock().expect("poisoned");
        conn.execute(
            "UPDATE pipeline_plans SET status = ?1 WHERE plan_id = ?2",
            params![format!("{:?}", status).to_lowercase(), plan_id],
        )?;
        Ok(())
    }

    // ─── CR-05 P3 slice 2 — recommendation persistence ──────────────

    /// CR-05 P3 slice 2 — insert or replace a `Recommendation` row.
    /// The full `ProcessingDecision` is serialised into
    /// `decision_json` so future rule additions don't require
    /// schema changes.
    pub fn insert_recommendation(
        &self,
        rec: &crate::recommendation::Recommendation,
    ) -> Result<(), PipelinePlanStoreError> {
        let decision_json = serde_json::to_string(&rec.decision)?;
        let conn = self.conn.lock().expect("poisoned");
        conn.execute(
            "INSERT OR REPLACE INTO recommendations
                (id, stage_execution_id, rule_id, stage_type,
                 decision_json, confidence, evidence_summary, created_at)
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            rusqlite::params![
                rec.id,
                rec.stage_execution_id,
                rec.rule_id,
                rec.decision.stage_type,
                decision_json,
                rec.confidence,
                rec.evidence_summary,
                rec.created_at,
            ],
        )?;
        Ok(())
    }

    /// CR-05 P3 slice 2 — bulk insert / replace recommendations.
    /// Wraps `insert_recommendation` in a single SQLite transaction
    /// so the engine's per-stage-evaluation output is atomic.
    pub fn insert_recommendations(
        &self,
        recs: &[crate::recommendation::Recommendation],
    ) -> Result<(), PipelinePlanStoreError> {
        let mut conn = self.conn.lock().expect("poisoned");
        let tx = conn.transaction()?;
        {
            let mut stmt = tx.prepare(
                "INSERT OR REPLACE INTO recommendations
                    (id, stage_execution_id, rule_id, stage_type,
                     decision_json, confidence, evidence_summary, created_at)
                    VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            )?;
            for rec in recs {
                let decision_json = serde_json::to_string(&rec.decision)?;
                stmt.execute(rusqlite::params![
                    rec.id,
                    rec.stage_execution_id,
                    rec.rule_id,
                    rec.decision.stage_type,
                    decision_json,
                    rec.confidence,
                    rec.evidence_summary,
                    rec.created_at,
                ])?;
            }
        }
        tx.commit()?;
        Ok(())
    }

    /// CR-05 P3 slice 2 — list recommendations emitted for a given
    /// `stage_execution_id`. Returns rows in stable `rule_id` order.
    pub fn list_recommendations_for_stage_execution(
        &self,
        stage_execution_id: &str,
    ) -> Result<Vec<crate::recommendation::Recommendation>, PipelinePlanStoreError> {
        let conn = self.conn.lock().expect("poisoned");
        let mut stmt = conn.prepare(
            "SELECT id, stage_execution_id, rule_id, decision_json,
                    confidence, evidence_summary, created_at,
                    user_decision, user_decision_at, applied_stage_id,
                    applied_with_preview_id
             FROM recommendations
             WHERE stage_execution_id = ?1
             ORDER BY rule_id ASC",
        )?;
        let rows = stmt.query_map([stage_execution_id], |row| {
            let id: String = row.get(0)?;
            let stage_execution_id: String = row.get(1)?;
            let rule_id: String = row.get(2)?;
            let decision_json: String = row.get(3)?;
            let confidence: f64 = row.get(4)?;
            let evidence_summary: String = row.get(5)?;
            let created_at: String = row.get(6)?;
            let user_decision: Option<String> = row.get(7)?;
            let user_decision_at: Option<String> = row.get(8)?;
            let applied_stage_id: Option<String> = row.get(9)?;
            let applied_with_preview_id: Option<String> = row.get(10)?;
            let decision: crate::recommendation::ProcessingDecision =
                serde_json::from_str(&decision_json).map_err(|e| {
                    rusqlite::Error::FromSqlConversionFailure(
                        3,
                        rusqlite::types::Type::Text,
                        Box::new(e),
                    )
                })?;
            Ok(crate::recommendation::Recommendation {
                id,
                stage_execution_id,
                rule_id,
                decision,
                confidence,
                evidence_summary,
                created_at,
                user_decision,
                user_decision_at,
                applied_stage_id,
                applied_with_preview_id,
            })
        })?;
        let mut out = Vec::new();
        for row in rows {
            out.push(row?);
        }
        Ok(out)
    }

    /// CR-05 P3 slice 2 — list recommendations for every stage
    /// execution that belongs to the given plan. Used by
    /// `get_recommendation(plan_id)` (Tauri command) so the
    /// IntelligencePanel can show all current picks at once.
    pub fn list_recommendations_for_plan(
        &self,
        plan_id: &str,
    ) -> Result<Vec<crate::recommendation::Recommendation>, PipelinePlanStoreError> {
        let conn = self.conn.lock().expect("poisoned");
        let mut stmt = conn.prepare(
            "SELECT r.id, r.stage_execution_id, r.rule_id, r.decision_json,
                    r.confidence, r.evidence_summary, r.created_at,
                    r.user_decision, r.user_decision_at, r.applied_stage_id
             FROM recommendations r
             JOIN stage_executions s ON s.stage_execution_id = r.stage_execution_id
             WHERE s.plan_id = ?1
             ORDER BY r.rule_id ASC",
        )?;
        let rows = stmt.query_map([plan_id], |row| {
            let id: String = row.get(0)?;
            let stage_execution_id: String = row.get(1)?;
            let rule_id: String = row.get(2)?;
            let decision_json: String = row.get(3)?;
            let confidence: f64 = row.get(4)?;
            let evidence_summary: String = row.get(5)?;
            let created_at: String = row.get(6)?;
            let user_decision: Option<String> = row.get(7)?;
            let user_decision_at: Option<String> = row.get(8)?;
            let applied_stage_id: Option<String> = row.get(9)?;
            let decision: crate::recommendation::ProcessingDecision =
                serde_json::from_str(&decision_json).map_err(|e| {
                    rusqlite::Error::FromSqlConversionFailure(
                        3,
                        rusqlite::types::Type::Text,
                        Box::new(e),
                    )
                })?;
            Ok(crate::recommendation::Recommendation {
                id,
                stage_execution_id,
                rule_id,
                decision,
                confidence,
                evidence_summary,
                created_at,
                user_decision,
                user_decision_at,
                applied_stage_id,
                // P4 slice 6 — list_recommendations_for_plan
                // doesn't carry the preview id column (it's only
                // populated when the user actually applies). The
                // SQL SELECT also omits it for the same reason.
                applied_with_preview_id: None,
            })
        })?;
        let mut out = Vec::new();
        for row in rows {
            out.push(row?);
        }
        Ok(out)
    }

    // ─── CR-05 P3 slice 2.5 — recommendation user-decision persistence ────

    /// CR-05 P3 slice 2.5 — resolve a recommendation's
    /// `stage_execution_id` by its primary key. Used by the
    /// apply / dismiss / reset Tauri commands to reload the row
    /// after the mutation commit.
    pub fn stage_execution_id_for_recommendation(
        &self,
        recommendation_id: &str,
    ) -> Result<String, PipelinePlanStoreError> {
        let conn = self.conn.lock().expect("poisoned");
        let stage_execution_id: String = conn
            .query_row(
                "SELECT stage_execution_id FROM recommendations WHERE id = ?1",
                [recommendation_id],
                |row| row.get(0),
            )
            .map_err(|e| match e {
                rusqlite::Error::QueryReturnedNoRows => {
                    PipelinePlanStoreError::NotFound(recommendation_id.to_string())
                }
                other => PipelinePlanStoreError::Sqlite(other),
            })?;
        Ok(stage_execution_id)
    }

    /// CR-05 P4 slice 5 — fetch a single stage execution row by id.
    /// The preview command needs this to resolve
    /// `stage_execution_id` → (plan_id, stage_id, parameters) before
    /// dispatching the preview driver.
    pub fn get_stage_execution(
        &self,
        stage_execution_id: &str,
    ) -> Result<StageExecution, PipelinePlanStoreError> {
        let conn = self.conn.lock().expect("poisoned");
        conn.query_row(
            "SELECT stage_execution_id, plan_id, stage_id, attempt, status, input_version_id, output_artifact_id, parameters_json, parameters_hash, started_at, completed_at, resource_usage_json, error_json, metric_snapshot_json
             FROM stage_executions WHERE stage_execution_id = ?1",
            params![stage_execution_id],
            |row| {
                Ok(StageExecution {
                    stage_execution_id: row.get(0)?,
                    plan_id: row.get(1)?,
                    stage_id: row.get(2)?,
                    attempt: row.get(3)?,
                    status: row.get(4)?,
                    input_version_id: row.get(5)?,
                    output_artifact_id: row.get(6)?,
                    parameters_json: row.get(7)?,
                    parameters_hash: row.get(8)?,
                    started_at: row.get(9)?,
                    completed_at: row.get(10)?,
                    resource_usage_json: row.get(11)?,
                    error_json: row.get(12)?,
                    metric_snapshot_json: row.get(13)?,
                })
            },
        )
        .map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => {
                PipelinePlanStoreError::NotFound(stage_execution_id.to_string())
            }
            other => PipelinePlanStoreError::Sqlite(other),
        })
    }

    /// CR-05 P4 slice 6 — fetch a single recommendation by id.
    /// Public wrapper around `get_recommendation_by_id_locked`
    /// so Tauri commands (which hold their own `&self` borrow)
    /// can resolve the row without juggling transactions.
    pub fn get_recommendation_by_id(
        &self,
        recommendation_id: &str,
    ) -> Result<Recommendation, PipelinePlanStoreError> {
        let conn = self.conn.lock().expect("poisoned");
        Self::get_recommendation_by_id_locked(&conn, recommendation_id)
    }

    /// CR-05 P3 slice 2.5 — resolve a recommendation row by id
    /// (without committing to a user_decision value yet). Used by
    /// the apply / dismiss / reset commands so they share the
    /// row-resolution path.
    fn get_recommendation_by_id_locked(
        conn: &rusqlite::Connection,
        id: &str,
    ) -> Result<crate::recommendation::Recommendation, PipelinePlanStoreError> {
        let mut stmt = conn.prepare(
            "SELECT id, stage_execution_id, rule_id, decision_json,
                    confidence, evidence_summary, created_at,
                    user_decision, user_decision_at, applied_stage_id,
                    applied_with_preview_id
             FROM recommendations WHERE id = ?1",
        )?;
        let mut rows = stmt.query([id])?;
        let row = rows
            .next()?
            .ok_or_else(|| PipelinePlanStoreError::NotFound(id.to_string()))?;
        let id: String = row.get(0)?;
        let stage_execution_id: String = row.get(1)?;
        let rule_id: String = row.get(2)?;
        let decision_json: String = row.get(3)?;
        let confidence: f64 = row.get(4)?;
        let evidence_summary: String = row.get(5)?;
        let created_at: String = row.get(6)?;
        let user_decision: Option<String> = row.get(7)?;
        let user_decision_at: Option<String> = row.get(8)?;
        let applied_stage_id: Option<String> = row.get(9)?;
        let applied_with_preview_id: Option<String> = row.get(10)?;
        let decision: crate::recommendation::ProcessingDecision =
            serde_json::from_str(&decision_json).map_err(|e| {
                rusqlite::Error::FromSqlConversionFailure(
                    3,
                    rusqlite::types::Type::Text,
                    Box::new(e),
                )
            })?;
        Ok(crate::recommendation::Recommendation {
            id,
            stage_execution_id,
            rule_id,
            decision,
            confidence,
            evidence_summary,
            created_at,
            user_decision,
            user_decision_at,
            applied_stage_id,
            applied_with_preview_id,
        })
    }

    /// CR-05 P3 slice 2.5 — find the next pipeline_stages row
    /// after the stage execution that emitted this recommendation
    /// (same plan, sequence > current stage's sequence). Returns
    /// `(stage_id, existing_parameters_json)`. Returns `None` when
    /// the recommendation is for the final stage in the plan.
    fn next_stage_after_locked(
        conn: &rusqlite::Connection,
        stage_execution_id: &str,
    ) -> Result<Option<(String, Option<String>)>, PipelinePlanStoreError> {
        let mut stmt = conn.prepare(
            "SELECT ps.stage_id, ps.parameters_json, ps.sequence, pse.plan_id
             FROM stage_executions pse
             JOIN pipeline_stages ps ON ps.plan_id = pse.plan_id AND ps.stage_id = pse.stage_id
             WHERE pse.stage_execution_id = ?1",
        )?;
        let mut rows = stmt.query([stage_execution_id])?;
        let Some(row) = rows.next()? else {
            return Ok(None);
        };
        let current_stage_id: String = row.get(0)?;
        let _current_params: Option<String> = row.get(1)?;
        let current_sequence: i64 = row.get(2)?;
        let plan_id: String = row.get(3)?;
        drop(rows);
        drop(stmt);
        let mut next_stmt = conn.prepare(
            "SELECT stage_id, parameters_json FROM pipeline_stages
             WHERE plan_id = ?1 AND sequence > ?2
             ORDER BY sequence ASC LIMIT 1",
        )?;
        let mut next_rows = next_stmt.query(params![plan_id, current_sequence])?;
        if let Some(next_row) = next_rows.next()? {
            let next_stage_id: String = next_row.get(0)?;
            let next_params: Option<String> = next_row.get(1)?;
            // Skip if the next stage is the one that produced this
            // recommendation (defensive — sequence > current_sequence
            // already excludes it, but keeps this branch explicit).
            if next_stage_id == current_stage_id {
                return Ok(None);
            }
            return Ok(Some((next_stage_id, next_params)));
        }
        Ok(None)
    }

    /// CR-05 P3 slice 2.5 — apply a recommendation: merge its
    /// recommended `parameters` into the next stage's
    /// `parameters_json` (preserving existing keys) and mark the
    /// recommendation as `applied`. Idempotent — a second call
    /// keeps the merged parameters and updates the timestamp.
    ///
    /// Returns `(recommendation_id, applied_stage_id)` so the
    /// frontend can confirm which stage consumed the override.
    ///
    /// **CR-05 P4 slice 6 — preview gate.** `applied_with_preview_id`
    /// must reference a real `preview_run.id` (typically the one
    /// the Tauri command just verified via
    /// `DomainStore::latest_completed_preview_for_stage_execution`).
    /// The store does not re-check the preview; the Tauri layer is
    /// the gate. Passing an empty string is allowed and means
    /// "no preview provenance recorded" — useful for tests and
    /// for migrations where no preview row exists yet.
    pub fn apply_recommendation(
        &self,
        recommendation_id: &str,
        timestamp: &str,
        applied_with_preview_id: &str,
    ) -> Result<(String, Option<String>), PipelinePlanStoreError> {
        let mut conn = self.conn.lock().expect("poisoned");
        let tx = conn.transaction()?;
        let rec = Self::get_recommendation_by_id_locked(&tx, recommendation_id)?;
        let next = Self::next_stage_after_locked(&tx, &rec.stage_execution_id)?;
        let applied_stage_id: Option<String> = if let Some((next_stage_id, existing)) = next {
            let mut merged: serde_json::Map<String, serde_json::Value> = match existing {
                Some(p) => serde_json::from_str(&p).unwrap_or_default(),
                None => serde_json::Map::new(),
            };
            for (k, v) in rec.decision.parameters.iter() {
                merged.insert(k.clone(), v.clone());
            }
            let merged_json = serde_json::to_string(&serde_json::Value::Object(merged))?;
            tx.execute(
                "UPDATE pipeline_stages SET parameters_json = ?1 WHERE stage_id = ?2",
                params![merged_json, next_stage_id],
            )?;
            Some(next_stage_id)
        } else {
            None
        };
        // Persist `applied_with_preview_id` alongside the other
        // apply metadata. NULL when the caller passed an empty
        // string (legacy / test path).
        let preview_id_persisted: Option<&str> = if applied_with_preview_id.is_empty() {
            None
        } else {
            Some(applied_with_preview_id)
        };
        tx.execute(
            "UPDATE recommendations
             SET user_decision = ?1, user_decision_at = ?2, applied_stage_id = ?3,
                 applied_with_preview_id = ?4
             WHERE id = ?5",
            params![
                crate::recommendation::user_decision::APPLIED,
                timestamp,
                applied_stage_id,
                preview_id_persisted,
                recommendation_id,
            ],
        )?;
        tx.commit()?;
        Ok((rec.id, applied_stage_id))
    }

    /// CR-05 P3 slice 2.5 — mark a recommendation as `dismissed`.
    /// Does NOT mutate the next stage's `parameters_json`. If the
    /// recommendation was previously applied, the previously-merged
    /// parameters are NOT rolled back here — that is left to a
    /// separate "reset" action (per CR-05 §27 the dismiss is a
    /// soft signal, not a rollback). Idempotent.
    pub fn dismiss_recommendation(
        &self,
        recommendation_id: &str,
        timestamp: &str,
    ) -> Result<String, PipelinePlanStoreError> {
        let mut conn = self.conn.lock().expect("poisoned");
        let tx = conn.transaction()?;
        let rec = Self::get_recommendation_by_id_locked(&tx, recommendation_id)?;
        tx.execute(
            "UPDATE recommendations
             SET user_decision = ?1, user_decision_at = ?2
             WHERE id = ?3",
            params![
                crate::recommendation::user_decision::DISMISSED,
                timestamp,
                recommendation_id,
            ],
        )?;
        tx.commit()?;
        Ok(rec.id)
    }

    /// CR-05 P3 slice 2.5 — reset a recommendation to `pending`.
    /// If it was previously applied, the previously-merged
    /// parameters are NOT un-merged from the next stage (the merge
    /// is a forward-only convenience; rolling back would require
    /// a snapshot of the prior `parameters_json`). The
    /// recommendation simply becomes available for re-apply.
    pub fn reset_recommendation(
        &self,
        recommendation_id: &str,
    ) -> Result<String, PipelinePlanStoreError> {
        let mut conn = self.conn.lock().expect("poisoned");
        let tx = conn.transaction()?;
        let rec = Self::get_recommendation_by_id_locked(&tx, recommendation_id)?;
        tx.execute(
            "UPDATE recommendations
             SET user_decision = ?1, user_decision_at = NULL, applied_stage_id = NULL
             WHERE id = ?2",
            params![
                crate::recommendation::user_decision::PENDING,
                recommendation_id
            ],
        )?;
        tx.commit()?;
        Ok(rec.id)
    }

    /// CR-05 P2.5 — list plans in `Paused` status (recovery candidates).
    /// Used by `find_resumable_runs_for_project` (Tauri command) to
    /// populate the `RecoveryBanner.svelte` UI on project open.
    pub fn list_resumable_plans_for_project(
        &self,
        project_id: &str,
    ) -> Result<Vec<PipelinePlanSummary>, PipelinePlanStoreError> {
        let conn = self.conn.lock().expect("poisoned");
        let mut stmt = conn.prepare(
            "SELECT plan_id, project_id, session_id, recipe_id, mode, target_type, status, created_at, schema_version
             FROM pipeline_plans WHERE project_id = ?1 AND status = 'paused' ORDER BY created_at DESC",
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

    // ─── Stage execution CRUD tests (CR-05 P2 slice 1) ────────────────

    use crate::domain::StageExecution;

    fn synthetic_entry(plan_id: &str, stage_id: &str, status: &str) -> StageExecution {
        StageExecution {
            stage_execution_id: format!("exec_{}_{}", plan_id, stage_id),
            plan_id: plan_id.into(),
            stage_id: stage_id.into(),
            attempt: 1,
            status: status.into(),
            input_version_id: None,
            output_artifact_id: None,
            parameters_json: None,
            parameters_hash: None,
            started_at: Some("unix_ms:100".into()),
            completed_at: None,
            resource_usage_json: None,
            error_json: None,
            metric_snapshot_json: None,
        }
    }

    #[test]
    fn stage_execution_round_trip() {
        let store = PipelinePlanStore::in_memory().unwrap();
        let exec = synthetic_entry("plan_1", "stage_calibrate", "running");
        store.insert_stage_execution(&exec).unwrap();

        let listed = store.list_stage_executions_for_plan("plan_1").unwrap();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].stage_id, "stage_calibrate");
        assert_eq!(listed[0].status, "running");
    }

    /// CR-05 P4 slice 5 — `get_stage_execution` fetches a single row
    /// by id; unknown ids return NotFound.
    #[test]
    fn get_stage_execution_round_trip_and_not_found() {
        let store = PipelinePlanStore::in_memory().unwrap();
        let exec = synthetic_entry("plan_1", "stage_calibrate", "completed");
        store.insert_stage_execution(&exec).unwrap();

        let fetched = store
            .get_stage_execution("exec_plan_1_stage_calibrate")
            .unwrap();
        assert_eq!(fetched.plan_id, "plan_1");
        assert_eq!(fetched.stage_id, "stage_calibrate");
        assert_eq!(fetched.status, "completed");

        let err = store.get_stage_execution("exec_missing").unwrap_err();
        match err {
            PipelinePlanStoreError::NotFound(id) => assert_eq!(id, "exec_missing"),
            other => panic!("expected NotFound, got {other:?}"),
        }
    }

    #[test]
    fn stage_execution_update_via_insert_or_replace() {
        let store = PipelinePlanStore::in_memory().unwrap();
        let mut exec = synthetic_entry("plan_1", "stage_calibrate", "running");
        store.insert_stage_execution(&exec).unwrap();
        exec.status = "completed".into();
        exec.completed_at = Some("unix_ms:200".into());
        store.update_stage_execution(&exec).unwrap();

        let listed = store.list_stage_executions_for_plan("plan_1").unwrap();
        assert_eq!(listed.len(), 1, "update should not duplicate the row");
        assert_eq!(listed[0].status, "completed");
        assert_eq!(listed[0].completed_at.as_deref(), Some("unix_ms:200"));
    }

    #[test]
    fn stage_execution_list_returns_in_started_order() {
        let store = PipelinePlanStore::in_memory().unwrap();
        let mut a = synthetic_entry("plan_1", "stage_a", "completed");
        a.started_at = Some("unix_ms:100".into());
        let mut b = synthetic_entry("plan_1", "stage_b", "completed");
        b.started_at = Some("unix_ms:200".into());
        let mut c = synthetic_entry("plan_1", "stage_c", "completed");
        c.started_at = Some("unix_ms:300".into());
        store.insert_stage_execution(&b).unwrap();
        store.insert_stage_execution(&c).unwrap();
        store.insert_stage_execution(&a).unwrap();

        let listed = store.list_stage_executions_for_plan("plan_1").unwrap();
        assert_eq!(listed.len(), 3);
        // Started-time ASC ordering: a (100), b (200), c (300).
        assert_eq!(listed[0].stage_id, "stage_a");
        assert_eq!(listed[1].stage_id, "stage_b");
        assert_eq!(listed[2].stage_id, "stage_c");
    }

    #[test]
    fn stage_executions_isolated_per_plan() {
        let store = PipelinePlanStore::in_memory().unwrap();
        store
            .insert_stage_execution(&synthetic_entry("plan_1", "stage_a", "completed"))
            .unwrap();
        store
            .insert_stage_execution(&synthetic_entry("plan_2", "stage_b", "completed"))
            .unwrap();

        let plan_1 = store.list_stage_executions_for_plan("plan_1").unwrap();
        let plan_2 = store.list_stage_executions_for_plan("plan_2").unwrap();
        assert_eq!(plan_1.len(), 1);
        assert_eq!(plan_2.len(), 1);
        assert_eq!(plan_1[0].plan_id, "plan_1");
        assert_eq!(plan_2[0].plan_id, "plan_2");
    }

    #[test]
    fn update_plan_status_changes_visible_status() {
        let store = PipelinePlanStore::in_memory().unwrap();
        let (stages, target) = deep_sky_osc_balanced();
        let mut plan = generate_plan(&ctx(), &stages, target).unwrap();
        plan.plan_id = "plan_status".into();
        store.insert_plan(&plan).unwrap();

        store
            .update_plan_status("plan_status", PipelinePlanStatus::Running)
            .unwrap();
        let loaded = store.load_plan("plan_status").unwrap();
        assert_eq!(loaded.status, PipelinePlanStatus::Running);

        store
            .update_plan_status("plan_status", PipelinePlanStatus::Cancelled)
            .unwrap();
        let loaded = store.load_plan("plan_status").unwrap();
        assert_eq!(loaded.status, PipelinePlanStatus::Cancelled);
    }

    // ─── CR-05 P2.5 — recovery query test ──────────────────────────────

    #[test]
    fn list_resumable_plans_filters_to_paused_only() {
        let store = PipelinePlanStore::in_memory().unwrap();
        let (stages, target) = deep_sky_osc_balanced();
        let mut p_paused = generate_plan(&ctx(), &stages, target).unwrap();
        p_paused.plan_id = "plan_paused".into();
        let mut p_running = generate_plan(&ctx(), &stages, target).unwrap();
        p_running.plan_id = "plan_running".into();
        let mut p_completed = generate_plan(&ctx(), &stages, target).unwrap();
        p_completed.plan_id = "plan_completed".into();
        store.insert_plan(&p_paused).unwrap();
        store.insert_plan(&p_running).unwrap();
        store.insert_plan(&p_completed).unwrap();

        store
            .update_plan_status("plan_paused", PipelinePlanStatus::Paused)
            .unwrap();
        store
            .update_plan_status("plan_running", PipelinePlanStatus::Running)
            .unwrap();
        store
            .update_plan_status("plan_completed", PipelinePlanStatus::Completed)
            .unwrap();

        let resumable = store.list_resumable_plans_for_project("proj_1").unwrap();
        assert_eq!(resumable.len(), 1);
        assert_eq!(resumable[0].plan_id, "plan_paused");
        assert_eq!(resumable[0].status, "paused");
    }

    // ─── CR-05 P3 slice 2 — recommendation persistence tests ─────────

    fn synthetic_recommendation(
        stage_execution_id: &str,
        rule_id: &str,
        stage_type: &str,
    ) -> crate::recommendation::Recommendation {
        let mut params = std::collections::BTreeMap::new();
        params.insert("k".into(), serde_json::json!(1));
        crate::recommendation::Recommendation {
            id: format!("rec_{}_{}", stage_execution_id, rule_id),
            stage_execution_id: stage_execution_id.into(),
            rule_id: rule_id.into(),
            decision: crate::recommendation::ProcessingDecision {
                stage_type: stage_type.into(),
                parameters: params,
                rationale: "test rationale".into(),
            },
            confidence: 0.75,
            evidence_summary: "mean=0.1 stddev=0.05".into(),
            created_at: "unix_ms:1".into(),
            user_decision: None,
            user_decision_at: None,
            applied_stage_id: None,
            applied_with_preview_id: None,
        }
    }

    #[test]
    fn recommendation_round_trip() {
        let store = PipelinePlanStore::in_memory().unwrap();
        let rec = synthetic_recommendation("exec_1", "stretch_v1", "stretch");
        store.insert_recommendation(&rec).unwrap();

        let loaded = store
            .list_recommendations_for_stage_execution("exec_1")
            .unwrap();
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0], rec);
    }

    #[test]
    fn bulk_recommendation_insert_is_atomic() {
        let store = PipelinePlanStore::in_memory().unwrap();
        let recs = vec![
            synthetic_recommendation("exec_2", "stretch_v1", "stretch"),
            synthetic_recommendation("exec_2", "denoise_v1", "denoise"),
            synthetic_recommendation("exec_2", "background_v1", "background"),
        ];
        store.insert_recommendations(&recs).unwrap();

        let loaded = store
            .list_recommendations_for_stage_execution("exec_2")
            .unwrap();
        assert_eq!(loaded.len(), 3);
        // Sort by rule_id (already done by store).
        let rule_ids: Vec<&str> = loaded.iter().map(|r| r.rule_id.as_str()).collect();
        let mut sorted = rule_ids.clone();
        sorted.sort();
        assert_eq!(rule_ids, sorted);
    }

    // ─── CR-05 P3 slice 2.5 — apply / dismiss / reset tests ─────────────

    /// CR-05 P3 slice 2.5 — build a plan with two stages and a
    /// stage execution for the first stage, so apply() can resolve
    /// a "next stage" to merge parameters into.
    fn plan_with_two_stages_and_execution(
        plan_id: &str,
        stage_a_id: &str,
        stage_b_id: &str,
        exec_a_id: &str,
        existing_b_params: Option<&str>,
    ) -> (PipelinePlan, StageExecution, StageExecution) {
        let plan = PipelinePlan {
            plan_id: plan_id.into(),
            project_id: "proj_1".into(),
            session_id: "sess_1".into(),
            recipe_id: None,
            mode: "guided".into(),
            target_type: ObjectType::DeepSky,
            status: PipelinePlanStatus::Running,
            created_at: "unix_ms:1".into(),
            schema_version: 1,
            stages: vec![
                PipelineStage {
                    stage_id: stage_a_id.into(),
                    plan_id: plan_id.into(),
                    sequence: 1,
                    label: "stretch".into(),
                    stage_type: "stretch".into(),
                    required: true,
                    enabled: true,
                    produces_image_version: true,
                    undo_supported: true,
                    parameters_json: None,
                },
                PipelineStage {
                    stage_id: stage_b_id.into(),
                    plan_id: plan_id.into(),
                    sequence: 2,
                    label: "denoise".into(),
                    stage_type: "denoise".into(),
                    required: true,
                    enabled: true,
                    produces_image_version: true,
                    undo_supported: true,
                    parameters_json: existing_b_params.map(|s| s.to_string()),
                },
            ],
        };
        let exec_a = StageExecution {
            stage_execution_id: exec_a_id.into(),
            plan_id: plan_id.into(),
            stage_id: stage_a_id.into(),
            attempt: 1,
            status: "completed".into(),
            input_version_id: None,
            output_artifact_id: None,
            parameters_json: None,
            parameters_hash: None,
            started_at: Some("unix_ms:1".into()),
            completed_at: Some("unix_ms:2".into()),
            resource_usage_json: None,
            error_json: None,
            metric_snapshot_json: None,
        };
        let exec_b = StageExecution {
            stage_execution_id: "exec_b".into(),
            plan_id: plan_id.into(),
            stage_id: stage_b_id.into(),
            attempt: 1,
            status: "pending".into(),
            input_version_id: None,
            output_artifact_id: None,
            parameters_json: existing_b_params.map(|s| s.to_string()),
            parameters_hash: None,
            started_at: None,
            completed_at: None,
            resource_usage_json: None,
            error_json: None,
            metric_snapshot_json: None,
        };
        (plan, exec_a, exec_b)
    }

    fn rec_with_params(
        exec_id: &str,
        rule_id: &str,
        stage_type: &str,
        params: Vec<(String, serde_json::Value)>,
    ) -> crate::recommendation::Recommendation {
        let mut parameters = std::collections::BTreeMap::new();
        for (k, v) in params {
            parameters.insert(k, v);
        }
        crate::recommendation::Recommendation {
            id: format!("rec_{}_{}", rule_id, exec_id),
            stage_execution_id: exec_id.into(),
            rule_id: rule_id.into(),
            decision: crate::recommendation::ProcessingDecision {
                stage_type: stage_type.into(),
                parameters,
                rationale: "test rationale".into(),
            },
            confidence: 0.75,
            evidence_summary: "mean=0.1".into(),
            created_at: "unix_ms:1".into(),
            user_decision: None,
            user_decision_at: None,
            applied_stage_id: None,
            applied_with_preview_id: None,
        }
    }

    #[test]
    fn p325_apply_merges_parameters_into_next_stage_and_marks_applied() {
        let store = PipelinePlanStore::in_memory().unwrap();
        let plan_id = "plan_p325_apply";
        let stage_a = "stage_a";
        let stage_b = "stage_b";
        let exec_a = "exec_a";
        let (plan, exec_a_row, _exec_b_row) =
            plan_with_two_stages_and_execution(plan_id, stage_a, stage_b, exec_a, None);
        store.insert_plan(&plan).unwrap();
        store.insert_stage_execution(&exec_a_row).unwrap();

        let rec = rec_with_params(
            exec_a,
            "stretch_v1",
            "stretch",
            vec![
                ("shadows".to_string(), serde_json::json!(0.1)),
                ("highlights".to_string(), serde_json::json!(0.9)),
            ],
        );
        store.insert_recommendation(&rec).unwrap();

        let (returned_id, applied_to) = store
            .apply_recommendation(&rec.id, "unix_ms:100", "")
            .unwrap();
        assert_eq!(returned_id, rec.id);
        assert_eq!(applied_to.as_deref(), Some(stage_b));

        // Recommendation row reflects the apply.
        let listed = store
            .list_recommendations_for_stage_execution(exec_a)
            .unwrap();
        assert_eq!(listed.len(), 1);
        assert_eq!(
            listed[0].user_decision.as_deref(),
            Some(crate::recommendation::user_decision::APPLIED)
        );
        assert_eq!(listed[0].user_decision_at.as_deref(), Some("unix_ms:100"));
        assert_eq!(listed[0].applied_stage_id.as_deref(), Some(stage_b));

        // Plan reload shows merged parameters on stage_b.
        let reloaded = store.load_plan(plan_id).unwrap();
        let stage_b_row = reloaded
            .stages
            .iter()
            .find(|s| s.stage_id == stage_b)
            .unwrap();
        let merged: serde_json::Map<String, serde_json::Value> =
            serde_json::from_str(stage_b_row.parameters_json.as_deref().unwrap()).unwrap();
        assert_eq!(merged.get("shadows"), Some(&serde_json::json!(0.1)));
        assert_eq!(merged.get("highlights"), Some(&serde_json::json!(0.9)));
    }

    #[test]
    fn p325_apply_preserves_existing_parameters_on_next_stage() {
        let store = PipelinePlanStore::in_memory().unwrap();
        let plan_id = "plan_p325_merge";
        let stage_a = "stage_a";
        let stage_b = "stage_b";
        let exec_a = "exec_a";
        let existing = r#"{"dip_amount":0.6, "blend_ratio":0.5}"#;
        let (plan, exec_a_row, _exec_b_row) =
            plan_with_two_stages_and_execution(plan_id, stage_a, stage_b, exec_a, Some(existing));
        store.insert_plan(&plan).unwrap();
        store.insert_stage_execution(&exec_a_row).unwrap();

        let rec = rec_with_params(
            exec_a,
            "denoise_v1",
            "denoise",
            vec![("dip_amount".to_string(), serde_json::json!(0.3))],
        );
        store.insert_recommendation(&rec).unwrap();

        store
            .apply_recommendation(&rec.id, "unix_ms:1", "")
            .unwrap();
        let reloaded = store.load_plan(plan_id).unwrap();
        let stage_b_row = reloaded
            .stages
            .iter()
            .find(|s| s.stage_id == stage_b)
            .unwrap();
        let merged: serde_json::Map<String, serde_json::Value> =
            serde_json::from_str(stage_b_row.parameters_json.as_deref().unwrap()).unwrap();
        // Override applied.
        assert_eq!(merged.get("dip_amount"), Some(&serde_json::json!(0.3)));
        // Existing keys preserved.
        assert_eq!(merged.get("blend_ratio"), Some(&serde_json::json!(0.5)));
    }

    /// CR-05 P4 slice 6 — `apply_recommendation` persists the
    /// `applied_with_preview_id` provenance column when a non-
    /// empty preview id is passed.
    #[test]
    fn p46_apply_records_preview_id_provenance() {
        let store = PipelinePlanStore::in_memory().unwrap();
        let plan_id = "plan_p46_preview_id";
        let stage_a = "stage_a";
        let stage_b = "stage_b";
        let exec_a = "exec_a";
        let (plan, exec_a_row, _exec_b) =
            plan_with_two_stages_and_execution(plan_id, stage_a, stage_b, exec_a, None);
        store.insert_plan(&plan).unwrap();
        store.insert_stage_execution(&exec_a_row).unwrap();
        let rec = rec_with_params(exec_a, "stretch_v1", "stretch", vec![]);
        store.insert_recommendation(&rec).unwrap();

        let (_id, _stage) = store
            .apply_recommendation(&rec.id, "unix_ms:42", "prev_abc123")
            .unwrap();

        let recs = store
            .list_recommendations_for_stage_execution(exec_a)
            .unwrap();
        assert_eq!(recs.len(), 1);
        assert_eq!(
            recs[0].applied_with_preview_id.as_deref(),
            Some("prev_abc123")
        );
    }

    /// CR-05 P4 slice 6 — `apply_recommendation` with an empty
    /// preview id stores NULL in `applied_with_preview_id`
    /// (legacy / test path). The apply still mutates the stage.
    #[test]
    fn p46_apply_with_empty_preview_id_persists_null() {
        let store = PipelinePlanStore::in_memory().unwrap();
        let plan_id = "plan_p46_empty";
        let stage_a = "stage_a";
        let stage_b = "stage_b";
        let exec_a = "exec_a";
        let (plan, exec_a_row, _exec_b) =
            plan_with_two_stages_and_execution(plan_id, stage_a, stage_b, exec_a, None);
        store.insert_plan(&plan).unwrap();
        store.insert_stage_execution(&exec_a_row).unwrap();
        let rec = rec_with_params(exec_a, "stretch_v1", "stretch", vec![]);
        store.insert_recommendation(&rec).unwrap();

        let (_id, _) = store
            .apply_recommendation(&rec.id, "unix_ms:42", "")
            .unwrap();

        let recs = store
            .list_recommendations_for_stage_execution(exec_a)
            .unwrap();
        assert_eq!(recs.len(), 1);
        assert!(recs[0].applied_with_preview_id.is_none());
    }

    #[test]
    fn p325_apply_is_idempotent_and_updates_timestamp() {
        let store = PipelinePlanStore::in_memory().unwrap();
        let plan_id = "plan_p325_idempotent";
        let stage_a = "stage_a";
        let stage_b = "stage_b";
        let exec_a = "exec_a";
        let (plan, exec_a_row, _) =
            plan_with_two_stages_and_execution(plan_id, stage_a, stage_b, exec_a, None);
        store.insert_plan(&plan).unwrap();
        store.insert_stage_execution(&exec_a_row).unwrap();

        let rec = rec_with_params(
            exec_a,
            "stretch_v1",
            "stretch",
            vec![("shadows".to_string(), serde_json::json!(0.1))],
        );
        store.insert_recommendation(&rec).unwrap();

        store
            .apply_recommendation(&rec.id, "unix_ms:1", "")
            .unwrap();
        store
            .apply_recommendation(&rec.id, "unix_ms:2", "")
            .unwrap();

        let listed = store
            .list_recommendations_for_stage_execution(exec_a)
            .unwrap();
        assert_eq!(
            listed[0].user_decision_at.as_deref(),
            Some("unix_ms:2"),
            "second apply updates timestamp"
        );
    }

    #[test]
    fn p325_dismiss_marks_dismissed_without_touching_stage_params() {
        let store = PipelinePlanStore::in_memory().unwrap();
        let plan_id = "plan_p325_dismiss";
        let stage_a = "stage_a";
        let stage_b = "stage_b";
        let exec_a = "exec_a";
        let (plan, exec_a_row, _) =
            plan_with_two_stages_and_execution(plan_id, stage_a, stage_b, exec_a, None);
        store.insert_plan(&plan).unwrap();
        store.insert_stage_execution(&exec_a_row).unwrap();

        let rec = rec_with_params(
            exec_a,
            "stretch_v1",
            "stretch",
            vec![("shadows".to_string(), serde_json::json!(0.1))],
        );
        store.insert_recommendation(&rec).unwrap();

        store.dismiss_recommendation(&rec.id, "unix_ms:5").unwrap();
        let listed = store
            .list_recommendations_for_stage_execution(exec_a)
            .unwrap();
        assert_eq!(
            listed[0].user_decision.as_deref(),
            Some(crate::recommendation::user_decision::DISMISSED)
        );
        assert_eq!(listed[0].user_decision_at.as_deref(), Some("unix_ms:5"));

        // Stage B parameters were not touched.
        let reloaded = store.load_plan(plan_id).unwrap();
        let stage_b_row = reloaded
            .stages
            .iter()
            .find(|s| s.stage_id == stage_b)
            .unwrap();
        assert!(stage_b_row.parameters_json.is_none());
    }

    #[test]
    fn p325_reset_clears_decision_and_timestamp() {
        let store = PipelinePlanStore::in_memory().unwrap();
        let plan_id = "plan_p325_reset";
        let stage_a = "stage_a";
        let stage_b = "stage_b";
        let exec_a = "exec_a";
        let (plan, exec_a_row, _) =
            plan_with_two_stages_and_execution(plan_id, stage_a, stage_b, exec_a, None);
        store.insert_plan(&plan).unwrap();
        store.insert_stage_execution(&exec_a_row).unwrap();

        let rec = rec_with_params(
            exec_a,
            "stretch_v1",
            "stretch",
            vec![("shadows".to_string(), serde_json::json!(0.1))],
        );
        store.insert_recommendation(&rec).unwrap();
        store
            .apply_recommendation(&rec.id, "unix_ms:7", "")
            .unwrap();
        store.reset_recommendation(&rec.id).unwrap();

        let listed = store
            .list_recommendations_for_stage_execution(exec_a)
            .unwrap();
        assert_eq!(
            listed[0].user_decision.as_deref(),
            Some(crate::recommendation::user_decision::PENDING)
        );
        assert!(listed[0].user_decision_at.is_none());
        assert!(listed[0].applied_stage_id.is_none());
    }

    #[test]
    fn p325_apply_returns_not_found_for_unknown_id() {
        let store = PipelinePlanStore::in_memory().unwrap();
        let err = store
            .apply_recommendation("rec_does_not_exist", "unix_ms:1", "")
            .expect_err("expected error");
        match err {
            PipelinePlanStoreError::NotFound(id) => assert_eq!(id, "rec_does_not_exist"),
            other => panic!("expected NotFound, got {other:?}"),
        }
    }

    #[test]
    fn p325_migration_is_idempotent_across_reopen() {
        let dir = std::env::temp_dir().join(format!(
            "astroforge_p325_migration_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("pipeline_plans.sqlite");
        // First open runs the migration.
        {
            let store = PipelinePlanStore::new(path.clone()).unwrap();
            drop(store);
        }
        // Second open must succeed and skip already-applied migrations.
        let store = PipelinePlanStore::new(path.clone()).unwrap();
        // Inserting a recommendation must still work — the new
        // columns are present.
        let rec = rec_with_params(
            "exec_a",
            "stretch_v1",
            "stretch",
            vec![("shadows".to_string(), serde_json::json!(0.1))],
        );
        store.insert_recommendation(&rec).unwrap();
        std::fs::remove_dir_all(&dir).ok();
    }

    fn synthetic_pipeline_plan(plan_id: &str) -> PipelinePlan {
        PipelinePlan {
            plan_id: plan_id.into(),
            project_id: "proj_1".into(),
            session_id: "sess_1".into(),
            recipe_id: None,
            mode: "guided".into(),
            target_type: crate::domain::ObjectType::DeepSky,
            status: crate::domain::PipelinePlanStatus::Running,
            created_at: "unix_ms:1".into(),
            schema_version: 1,
            stages: vec![PipelineStage {
                stage_id: "stage_1".into(),
                plan_id: plan_id.into(),
                sequence: 1,
                label: "stretch".into(),
                stage_type: "stretch".into(),
                required: true,
                enabled: true,
                produces_image_version: true,
                undo_supported: true,
                parameters_json: None,
            }],
        }
    }

    fn synthetic_stage_execution(plan_id: &str, stage_execution_id: &str) -> StageExecution {
        StageExecution {
            stage_execution_id: stage_execution_id.into(),
            plan_id: plan_id.into(),
            stage_id: stage_execution_id.into(),
            attempt: 1,
            status: "completed".into(),
            input_version_id: None,
            output_artifact_id: None,
            parameters_json: None,
            parameters_hash: None,
            started_at: Some("unix_ms:1".into()),
            completed_at: Some("unix_ms:2".into()),
            resource_usage_json: None,
            error_json: None,
            metric_snapshot_json: None,
        }
    }

    #[test]
    fn list_recommendations_for_plan_filters_by_plan() {
        let store = PipelinePlanStore::in_memory().unwrap();
        store
            .insert_plan(&synthetic_pipeline_plan("plan_x"))
            .unwrap();
        store
            .insert_plan(&synthetic_pipeline_plan("plan_y"))
            .unwrap();
        store
            .insert_stage_execution(&synthetic_stage_execution("plan_x", "exec_a"))
            .unwrap();
        store
            .insert_stage_execution(&synthetic_stage_execution("plan_x", "exec_b"))
            .unwrap();
        store
            .insert_stage_execution(&synthetic_stage_execution("plan_y", "exec_c"))
            .unwrap();
        store
            .insert_recommendation(&synthetic_recommendation("exec_a", "stretch_v1", "stretch"))
            .unwrap();
        store
            .insert_recommendation(&synthetic_recommendation("exec_b", "denoise_v1", "denoise"))
            .unwrap();
        store
            .insert_recommendation(&synthetic_recommendation("exec_c", "stretch_v1", "stretch"))
            .unwrap();

        let plan_x_recs = store.list_recommendations_for_plan("plan_x").unwrap();
        assert_eq!(plan_x_recs.len(), 2);
        let plan_y_recs = store.list_recommendations_for_plan("plan_y").unwrap();
        assert_eq!(plan_y_recs.len(), 1);
    }
}
