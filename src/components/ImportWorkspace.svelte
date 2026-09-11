<!--
  CR-03 P4 — Import workspace content.

  §12 says: "Import is the moment the user brings new data into a
  project. The wizard drives the user through Source Selection,
  Calibration Selection, Plate Solve, and Stacking Result. P4
  ships the workspace layout + the §26 first-time empty state.
  The actual import controls land in P4b alongside the
  wizard deprecation."

  CR-04 (Intelligent Import + Session Understanding + Target
  Detection) extends this surface with the auto-detect flow.

  CR-04 P9 — wires the new 4-step Import Wizard (§12) into
  the workspace shell. The wizard owns its own state via
  `state/import-wizard.ts`.
-->
<script lang="ts">
  import WorkspaceScreen from "./WorkspaceScreen.svelte";
  import ImportWizard from "./import-wizard/ImportWizard.svelte";
</script>

<WorkspaceScreen
  title="Import"
  icon="upload_file"
  description="Bring new data into the project. Drop a folder of captures, calibration frames, or stacked results and AstroForge handles the rest."
>
  <ImportWizard />
</WorkspaceScreen>
