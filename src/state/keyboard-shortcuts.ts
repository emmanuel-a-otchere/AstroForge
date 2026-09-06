// CR-03 P6 — keyboard shortcuts store.
//
// Centralized keyboard handler. P6 wires:
//   - Cmd/Ctrl + N       → trigger "create project" dialog
//   - Cmd/Ctrl + O       → jump to Projects nav target
//   - Cmd/Ctrl + S       → mark active project as saved
//   - Cmd/Ctrl + 1..6    → jump to studio workspace tab (overview /
//                          import / process / enhance / compare /
//                          export)
//   - Escape             → close dialog (consumed by dialogs
//                          themselves); close project when no
//                          dialog is open
//
// P6 also surfaces a small bottom-fixed toast (the existing
// `action-error` styling is reused) when a shortcut is fired, so
// the user can see what happened even if there's no immediate
// visible change.

import { onMount } from "svelte";
import { get } from "svelte/store";
import { studioViewport, type StudioView } from "./application";
import { applicationNavTarget } from "./application";
import { projectContext } from "./project-context";
import { dialogOpen, deleteDialogOpen } from "./dialog-state";

interface ShortcutCallbacks {
  onCreateProject?: () => void;
}

const STUDIO_TABS: StudioView[] = [
  "overview",
  "import",
  "process",
  "enhance",
  "compare",
  "export",
];

export function useKeyboardShortcuts(callbacks: ShortcutCallbacks) {
  onMount(() => {
    function isMetaOrCtrl(event: KeyboardEvent): boolean {
      return event.metaKey || event.ctrlKey;
    }

    function isTypingTarget(target: EventTarget | null): boolean {
      if (!(target instanceof HTMLElement)) return false;
      const tag = target.tagName.toLowerCase();
      if (tag === "input" || tag === "textarea" || tag === "select") return true;
      if (target.isContentEditable) return true;
      return false;
    }

    function handle(event: KeyboardEvent) {
      // Always handle Escape — it should close dialogs/project
      // even when focus is inside an input.
      if (event.key === "Escape") {
        if (get(dialogOpen)) return; // dialogs handle their own Escape
        if (get(deleteDialogOpen)) return; // ditto
        if (get(projectContext) !== null) {
          projectContext.close();
          studioViewport.closeProject();
        }
        return;
      }

      // For shortcuts, skip when typing in inputs (except when
      // explicitly holding the modifier — Cmd/Ctrl combos
      // typically fire while in inputs, but it's safer to let
      // the browser handle native shortcuts first).
      if (isTypingTarget(event.target)) {
        return;
      }

      if (!isMetaOrCtrl(event)) return;

      switch (event.key.toLowerCase()) {
        case "n":
          event.preventDefault();
          callbacks.onCreateProject?.();
          return;
        case "o":
          event.preventDefault();
          applicationNavTarget.set("projects");
          return;
        case "s":
          event.preventDefault();
          if (get(projectContext) !== null) {
            projectContext.markDirty(false);
          }
          return;
        case "1":
        case "2":
        case "3":
        case "4":
        case "5":
        case "6": {
          if (get(projectContext) === null) return;
          const idx = Number.parseInt(event.key, 10) - 1;
          const view = STUDIO_TABS[idx];
          if (view) {
            event.preventDefault();
            studioViewport.setView(view);
          }
          return;
        }
      }
    }

    window.addEventListener("keydown", handle);
    return () => window.removeEventListener("keydown", handle);
  });
}