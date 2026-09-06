// CR-03 P1 — application navigation store.
//
// P1 introduces the *Application* navigation context (Home / Projects /
// Recipes / AI Models / Settings / Help). P3 introduces the Studio
// overlay, which is layered on top of the application shell when a
// Project is open. P1 ships the application half only; the studio
// overlay is wired in P3.

import { writable } from "svelte/store";

export type ApplicationNavTarget =
  | "home"
  | "projects"
  | "recipes"
  | "ai-models"
  | "settings"
  | "help";

export const APPLICATION_NAV_TARGETS: readonly ApplicationNavTarget[] = [
  "home",
  "projects",
  "recipes",
  "ai-models",
  "settings",
  "help",
] as const;

export interface ApplicationNavItem {
  id: ApplicationNavTarget;
  label: string;
  icon: string;
  /** Hint shown beside the icon. Empty when none. */
  hint: string;
}

export const APPLICATION_NAV_ITEMS: readonly ApplicationNavItem[] = [
  { id: "home", label: "Home", icon: "home", hint: "What do I want to do?" },
  {
    id: "projects",
    label: "Projects",
    icon: "folder",
    hint: "Open a saved astrophotography library.",
  },
  {
    id: "recipes",
    label: "Recipes",
    icon: "bookmark",
    hint: "Reusable processing workflows.",
  },
  {
    id: "ai-models",
    label: "AI Models",
    icon: "auto_awesome",
    hint: "Manage installed AI capabilities.",
  },
  {
    id: "settings",
    label: "Settings",
    icon: "tune",
    hint: "Application preferences.",
  },
  { id: "help", label: "Help", icon: "help", hint: "Docs and shortcuts." },
] as const;

function isApplicationNavTarget(value: string): value is ApplicationNavTarget {
  return (APPLICATION_NAV_TARGETS as readonly string[]).includes(value);
}

const initial: ApplicationNavTarget = "home";

const internal = writable<ApplicationNavTarget>(initial);

export const applicationNavTarget = {
  subscribe: internal.subscribe,
  set(target: ApplicationNavTarget) {
    internal.set(target);
  },
  /** Safe set that ignores unknown values. Used by URL/deep-link code in later phases. */
  setFromString(value: string) {
    if (isApplicationNavTarget(value)) {
      internal.set(value);
    }
  },
  reset() {
    internal.set(initial);
  },
};

export function applicationNavItem(
  target: ApplicationNavTarget,
): ApplicationNavItem {
  const found = APPLICATION_NAV_ITEMS.find((i) => i.id === target);
  if (!found) throw new Error(`unknown application nav target: ${target}`);
  return found;
}

/** Reactive read for components that prefer not to subscribe directly. */
export function readApplicationNavTarget(): ApplicationNavTarget {
  let current: ApplicationNavTarget = initial;
  internal.subscribe((v) => (current = v))();
  return current;
}

// ─── Studio viewport (CR-03 P3) ──────────────────────────────────────────
//
// The Studio viewport is a layered overlay (CR-03 §4 + §10). When a
// project is open, the Studio renders on top of the application
// shell with its own header + tab rail + workspace. P3 ships the
// types and a separate store; the ApplicationShell does not yet
// consume them — that lands in P3 wiring.

export type StudioView =
  | "overview"
  | "import"
  | "process"
  | "enhance"
  | "compare"
  | "export";

export const STUDIO_VIEWS: readonly StudioView[] = [
  "overview",
  "import",
  "process",
  "enhance",
  "compare",
  "export",
] as const;

export interface StudioNavItem {
  id: StudioView;
  label: string;
  icon: string;
}

export const STUDIO_NAV_ITEMS: readonly StudioNavItem[] = [
  { id: "overview", label: "Overview", icon: "dashboard" },
  { id: "import", label: "Import", icon: "upload_file" },
  { id: "process", label: "Process", icon: "auto_fix_high" },
  { id: "enhance", label: "Enhance", icon: "tune" },
  { id: "compare", label: "Compare", icon: "compare" },
  { id: "export", label: "Export", icon: "download" },
] as const;

function isStudioView(value: string): value is StudioView {
  return (STUDIO_VIEWS as readonly string[]).includes(value);
}

interface StudioState {
  /** Currently-open project (null = no project open). */
  project: { project_id: string; name: string } | null;
  /** Active tab inside the studio. */
  view: StudioView;
}

const studioInternal = writable<StudioState>({
  project: null,
  view: "overview",
});

export const studioViewport = {
  subscribe: studioInternal.subscribe,
  /** Open a project in the Studio. Sets the active view to overview. */
  openProject(project: { project_id: string; name: string }) {
    studioInternal.set({ project, view: "overview" });
  },
  /** Close the active project. */
  closeProject() {
    studioInternal.set({ project: null, view: "overview" });
  },
  /** Set the active tab inside the studio. */
  setView(view: StudioView) {
    studioInternal.update((s) => ({ ...s, view }));
  },
  setViewFromString(value: string) {
    if (isStudioView(value)) {
      studioInternal.update((s) => ({ ...s, view: value }));
    }
  },
  reset() {
    studioInternal.set({ project: null, view: "overview" });
  },
};