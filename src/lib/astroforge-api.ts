// src/lib/astroforge-api.ts
//
// CR-02.6 — frontend wrapper for the durable Project / Pipeline-Run
// service layer. **This module is additive only.** Existing widgets
// continue to read from `sessionStore`, `gallery.ts`, etc. Nothing in
// this file wires any Svelte component to the Tauri commands — it just
// provides typed wrappers so future UI migration PRs have a single
// place to import from.
//
// The PR's single rule:
//   *"add the IPC surface; do not replace existing readers."*
//
// Future migration shape:
//   - Today: components import { createInitialSession } from "./pipeline-store"
//   - Tomorrow: components also import { createProject } from "./astroforge-api"
//     and the store reads project state through this layer.
//
// Each method here is a thin typed wrapper around `invoke()`. Errors
// surface to the caller as a thrown Error with the Rust message — the
// UI migrator is responsible for surfacing to the user.

import { invoke } from "@tauri-apps/api/core";

// ─── Project types (mirror Rust serde structs in commands_project.rs) ──

export type ProjectStatus = "Active" | "Archived" | "Exported";

export interface ProjectSummary {
  project_id: string;
  name: string;
  status: ProjectStatus;
  created_at: string;
  updated_at: string;
  active_session_id: string | null;
  application_version: string;
}

export type PipelineRunStatus =
  | "Queued"
  | "Running"
  | "Completed"
  | "Failed"
  | "Cancelled";

export interface PipelineRunSummary {
  run_id: string;
  project_id: string;
  status: PipelineRunStatus;
  recipe_id: string | null;
  started_at: string | null;
  completed_at: string | null;
}

export interface StageRunSummary {
  stage_run_id: string;
  run_id: string;
  stage_id: string;
  status: string;
  attempt: number;
  started_at: string | null;
  completed_at: string | null;
}

export interface RecoverSummary {
  project_id: string;
  status: ProjectStatus;
  repaired_dirs: string[];
}

// ─── Project lifecycle commands ────────────────────────────────────────

export const projectList = (): Promise<ProjectSummary[]> =>
  invoke("project_list");

export const projectGet = (projectId: string): Promise<ProjectSummary> =>
  invoke("project_get", { projectId });

export interface CreateProjectArgs {
  name: string;
  targetId?: string;
  applicationVersion: string;
}

export const projectCreate = (args: CreateProjectArgs): Promise<ProjectSummary> =>
  invoke("project_create", {
    name: args.name,
    targetId: args.targetId ?? null,
    applicationVersion: args.applicationVersion,
  });

export const projectOpen = (slug: string): Promise<ProjectSummary> =>
  invoke("project_open", { slug });

export const projectRename = (slug: string, newName: string): Promise<ProjectSummary> =>
  invoke("project_rename", { slug, newName });

export const projectArchive = (slug: string): Promise<void> =>
  invoke("project_archive", { slug });

export const projectDelete = (slug: string): Promise<void> =>
  // CR-02 §26 hard-delete gate — must be explicit on the wire.
  invoke("project_delete", { slug, confirm: true });

export const projectRecover = (slug: string): Promise<RecoverSummary> =>
  invoke("project_recover", { slug });

// ─── Pipeline-run commands (read-only in this scaffold) ────────────────

export const pipelineRunList = (projectId: string): Promise<PipelineRunSummary[]> =>
  invoke("pipeline_run_list", { projectId });

export const pipelineRunGet = (runId: string): Promise<PipelineRunSummary> =>
  invoke("pipeline_run_get", { runId });

export const pipelineRunListStages = (runId: string): Promise<StageRunSummary[]> =>
  invoke("pipeline_run_list_stages", { runId });

export const pipelineRunFindInterrupted = (): Promise<PipelineRunSummary[]> =>
  invoke("pipeline_run_find_interrupted");