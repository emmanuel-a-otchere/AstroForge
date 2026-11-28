<template>
  <ApplicationShell :project-label="appStore.projectLabel">
    <StudioShell v-if="appStore.hasOpenProject" />
    <router-view v-else @action="handleAction" />
  </ApplicationShell>

  <UpdateBanner />

  <ProjectDialog
    v-if="dialogs.dialogOpen"
    :mode="dialogs.dialogMode"
    :project="dialogs.dialogProject ?? undefined"
    :open="dialogs.dialogOpen"
    @submit="handleDialogSubmit"
    @cancel="dialogs.closeDialog"
  />

  <DeleteProjectDialog
    v-if="dialogs.deleteDialogOpen && dialogs.deleteTarget"
    :project="dialogs.deleteTarget"
    :open="dialogs.deleteDialogOpen"
    @confirm="handleDeleteConfirm"
    @cancel="dialogs.closeDelete"
  />

  <div v-if="dialogs.dialogError" class="action-error" role="alert">
    <span class="material-symbols-outlined" aria-hidden="true">error</span>
    <span>{{ dialogs.dialogError }}</span>
  </div>
</template>

<script setup>
import { onMounted } from "vue";
import { useRouter } from "vue-router";
import ApplicationShell from "@/components/ApplicationShell.vue";
import StudioShell from "@/components/StudioShell.vue";
import UpdateBanner from "@/components/UpdateBanner.vue";
import ProjectDialog from "@/components/ProjectDialog.vue";
import DeleteProjectDialog from "@/components/DeleteProjectDialog.vue";
import { useApplicationStore } from "@/stores/application";
import { useProjectsStore } from "@/stores/projects";
import { useDialogStore } from "@/stores/dialog";
import { useLifecycle } from "@/stores/lifecycle";
import { useKeyboardShortcuts } from "@/composables/useKeyboardShortcuts";
import * as api from "@/lib/astroforge-api";
import { slugFromName } from "@/common/utils";

const appStore = useApplicationStore();
const projects = useProjectsStore();
const dialogs = useDialogStore();
const { openProject } = useLifecycle();
const router = useRouter();

onMounted(() => {
  void projects.refresh();
});

useKeyboardShortcuts({
  onCreateProject: () => dialogs.openCreate(),
});

function handleAction(action, project) {
  if (action === "new-project" || action === "create") {
    dialogs.openCreate();
    return;
  }
  if (action === "open-project") {
    if (project) {
      const summary = projects.items.find((p) => p.name === project.name);
      if (summary) {
        void openProject(summary);
        void router.push("/studio");
        return;
      }
    }
    appStore.setNav("projects");
    void router.push("/projects");
    return;
  }
  if (action === "import-session") {
    if (appStore.hasOpenProject) {
      appStore.setView("import");
      void router.push("/studio");
    } else {
      dialogs.openCreate("import");
    }
    return;
  }
  if (action === "resume-processing") {
    if (appStore.hasOpenProject) {
      appStore.setView("process");
      void router.push("/studio");
    } else {
      appStore.setNav("projects");
      void router.push("/projects");
    }
    return;
  }
  if (action === "open" && project) {
    void openProject(project);
    void router.push("/studio");
    return;
  }
  if (action === "rename" && project) {
    dialogs.openRename(project);
    return;
  }
  if (action === "delete" && project) {
    dialogs.openDelete(project);
    return;
  }
  if (action === "archive" && project) {
    api
      .projectArchive(slugFromName(project.name))
      .then(() => projects.refresh())
      .catch((e) => (dialogs.dialogError = e instanceof Error ? e.message : String(e)));
    return;
  }
}

async function handleDialogSubmit(value) {
  try {
    if (dialogs.dialogMode === "create") {
      const created = await api.projectCreate({
        name: value.name,
        applicationVersion: "0.1.0",
      });
      dialogs.closeDialog();
      await projects.refresh();
      await openProject(created);
      void router.push("/studio");
      if (dialogs.postCreateAction === "import") {
        appStore.setView("import");
        dialogs.postCreateAction = null;
      }
    } else if (dialogs.dialogMode === "rename" && dialogs.dialogProject) {
      await api.projectRename(slugFromName(dialogs.dialogProject.name), value.name);
      dialogs.closeDialog();
      await projects.refresh();
    }
  } catch (e) {
    dialogs.dialogError = e instanceof Error ? e.message : String(e);
  }
}

async function handleDeleteConfirm() {
  if (!dialogs.deleteTarget) return;
  const slug = slugFromName(dialogs.deleteTarget.name);
  try {
    await api.projectDelete(slug);
    dialogs.closeDelete();
    await projects.refresh();
  } catch (e) {
    dialogs.dialogError = e instanceof Error ? e.message : String(e);
    dialogs.closeDelete();
  }
}
</script>

<style scoped>
.action-error {
  position: fixed;
  bottom: var(--sp-xl);
  left: 50%;
  transform: translateX(-50%);
  display: flex;
  align-items: center;
  gap: var(--sp-sm);
  padding: var(--sp-sm) var(--sp-lg);
  background: var(--surface-container-highest);
  border: 1px solid #e53935;
  border-radius: var(--radius-lg);
  color: var(--on-surface);
  z-index: 1000;
}

.action-error .material-symbols-outlined {
  color: #ff8a80;
}
</style>
