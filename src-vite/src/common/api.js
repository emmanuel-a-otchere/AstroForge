import { invokeCommand } from "./tauri.js";

/**
 * @typedef {"Active" | "Archived" | "Exported"} ProjectStatus
 * @typedef {{ project_id: string, name: string, status: ProjectStatus,
 *   application_version?: string, started_at?: string | null,
 *   completed_at?: string | null, updated_at?: string,
 *   active_session_id?: string | null }} ProjectSummary
 * @typedef {{ project_id: string, status: ProjectStatus, repaired_dirs: string[] }} RecoverSummary
 */

/** @returns {Promise<ProjectSummary[]>} */
export const projectList = () => invokeCommand("project_list");

/** @returns {Promise<ProjectSummary>} */
export const projectGet = (projectId) => invokeCommand("project_get", { projectId });

/**
 * @param {{ name: string, targetId?: string, applicationVersion: string }} args
 * @returns {Promise<ProjectSummary>}
 */
export const projectCreate = (args) =>
  invokeCommand("project_create", {
    name: args.name,
    targetId: args.targetId ?? null,
    applicationVersion: args.applicationVersion,
  });

/** @returns {Promise<ProjectSummary>} */
export const projectOpen = (slug) => invokeCommand("project_open", { slug });

/** @returns {Promise<ProjectSummary>} */
export const projectRename = (slug, newName) =>
  invokeCommand("project_rename", { slug, newName });

/** @returns {Promise<void>} */
export const projectArchive = (slug) => invokeCommand("project_archive", { slug });

/** @returns {Promise<void>} */
export const projectDelete = (slug) =>
  invokeCommand("project_delete", { slug, confirm: true });

/** @returns {Promise<RecoverSummary>} */
export const projectRecover = (slug) => invokeCommand("project_recover", { slug });
