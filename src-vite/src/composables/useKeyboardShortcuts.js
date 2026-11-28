import { onBeforeUnmount, onMounted } from "vue";
import { useApplicationStore, STUDIO_VIEWS } from "@/stores/application";
import { useProjectContextStore } from "@/stores/projectContext";
import { useDialogStore } from "@/stores/dialog";
import { useLifecycle } from "@/stores/lifecycle";

export function useKeyboardShortcuts(callbacks = {}) {
  const app = useApplicationStore();
  const context = useProjectContextStore();
  const dialogs = useDialogStore();
  const { closeProject } = useLifecycle();

  function isMetaOrCtrl(event) {
    return event.metaKey || event.ctrlKey;
  }

  function isTypingTarget(target) {
    if (!(target instanceof HTMLElement)) return false;
    const tag = target.tagName.toLowerCase();
    if (tag === "input" || tag === "textarea" || tag === "select") return true;
    if (target.isContentEditable) return true;
    return false;
  }

  function handle(event) {
    if (event.key === "Escape") {
      if (dialogs.dialogOpen || dialogs.deleteDialogOpen) return;
      if (app.hasOpenProject) closeProject();
      return;
    }
    if (isTypingTarget(event.target)) return;
    if (!isMetaOrCtrl(event)) return;

    switch (event.key.toLowerCase()) {
      case "n":
        event.preventDefault();
        callbacks.onCreateProject?.();
        return;
      case "o":
        event.preventDefault();
        app.setNav("projects");
        return;
      case "s":
        event.preventDefault();
        if (app.hasOpenProject) context.markDirty(false);
        return;
      case "1":
      case "2":
      case "3":
      case "4":
      case "5":
      case "6": {
        if (!app.hasOpenProject) return;
        const idx = Number.parseInt(event.key, 10) - 1;
        const view = STUDIO_VIEWS[idx];
        if (view) {
          event.preventDefault();
          app.setView(view);
        }
        return;
      }
    }
  }

  onMounted(() => window.addEventListener("keydown", handle));
  onBeforeUnmount(() => window.removeEventListener("keydown", handle));
}
