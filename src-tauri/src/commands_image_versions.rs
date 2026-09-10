//! CR-05 R3 — image version listing IPC.
//!
//! Surfaces the project's image-version timeline for the Compare
//! workspace. The canonical `image_versions` table does not yet
//! exist (it is on the audit roadmap as a follow-up to the
//! `Artifact` migration), so the IPC derives the timeline from the
//! durable event log: any `VersionCreated` event for the project
//! is a version row. When the table ships, the derivation in
//! this file becomes a fallback.
//!
//! Each row mirrors `astroforge_core::domain::ImageVersion` so the
//! Svelte side can render the same shape it already accepts.

use astroforge_core::domain::ImageVersion;

#[derive(Debug, Clone, serde::Serialize)]
pub struct ImageVersionDto {
    pub version_id: String,
    pub project_id: String,
    pub label: String,
    pub sequence: u32,
    /// Best-effort: the event-log payload's `primary_artifact_id`
    /// when present, else empty. The Compare workspace uses this
    /// to know whether a real artifact is attached.
    pub primary_artifact_id: String,
    pub source_version_id: Option<String>,
    pub created_at: String,
    pub hidden: bool,
}

impl From<ImageVersion> for ImageVersionDto {
    fn from(v: ImageVersion) -> Self {
        Self {
            version_id: v.version_id,
            project_id: v.project_id,
            label: v.label,
            sequence: v.sequence,
            primary_artifact_id: v.primary_artifact_id,
            source_version_id: v.source_version_id,
            created_at: v.created_at,
            hidden: v.hidden,
        }
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ImageVersionListResponse {
    pub project_id: String,
    pub versions: Vec<ImageVersionDto>,
}

/// List the image-version timeline for a project. Read-only,
/// infallible for known projects. The list is sorted by `sequence`
/// ascending so the Svelte side can rely on the order.
#[tauri::command]
pub fn image_version_list(
    state: State<'_, crate::commands_project::ProjectState>,
    project_id: String,
) -> Result<ImageVersionListResponse, String> {
    use astroforge_core::domain::ProjectEventKind;

    let store = state.store.lock().map_err(crate::commands_project::lock_err)?;
    // Touch the project so an unknown id errors with a clear
    // "project not found" path rather than an empty list.
    let project = store
        .get_project(&project_id)
        .map_err(crate::commands_project::store_err_to_string)?;

    let events = store
        .list_events(&project_id)
        .map_err(crate::commands_project::store_err_to_string)?;

    let mut versions: Vec<ImageVersionDto> = Vec::new();
    let mut next_seq: u32 = 0;
    for ev in &events {
        if ev.kind != ProjectEventKind::VersionCreated {
            continue;
        }
        // payload_json, when set, carries the version metadata.
        // Older writes may not include a payload; treat missing
        // fields as honest "unknown" (empty strings, sequence
        // derived from event order).
        let (label, artifact_id) = match ev.payload_json.as_deref() {
            Some(s) => parse_version_payload(s),
            None => (String::new(), String::new()),
        };
        versions.push(ImageVersionDto {
            version_id: ev.event_id.clone(),
            project_id: project.project_id.clone(),
            label: if label.is_empty() {
                format!("v{}", next_seq + 1)
            } else {
                label
            },
            sequence: next_seq,
            primary_artifact_id: artifact_id,
            source_version_id: None,
            created_at: ev.created_at.clone(),
            hidden: false,
        });
        next_seq += 1;
    }

    Ok(ImageVersionListResponse {
        project_id: project.project_id,
        versions,
    })
}

/// Decode the event payload. The schema is intentionally
/// forgiving: unknown fields are ignored, missing fields default
/// to empty. The shape is the minimum needed for the Compare
/// workspace to render real, durable state.
fn parse_version_payload(s: &str) -> (String, String) {
    #[derive(serde::Deserialize)]
    struct V {
        #[serde(default)]
        label: String,
        #[serde(default)]
        primary_artifact_id: String,
    }
    serde_json::from_str::<V>(s)
        .map(|v| (v.label, v.primary_artifact_id))
        .unwrap_or_default()
}
