//! P0 smoke tests: the `api::project` slice over a throwaway data dir
//! mirrors the Tauri command bodies (including the #465 W1.5 global
//! write-through for freshly created projects).

use astroforge_ui::api::project;
use astroforge_ui::services::Services;

fn temp_services() -> (Services, tempfile::TempDir) {
    let dir = tempfile::tempdir().expect("tempdir");
    let services = Services::open(dir.path()).expect("services open");
    (services, dir)
}

#[test]
fn create_then_list_and_get_roundtrip() {
    let (services, _keep) = temp_services();

    let created =
        project::project_create(&services, "Andromeda Test", None, env!("CARGO_PKG_VERSION"))
            .expect("create project");

    assert_eq!(created.name, "Andromeda Test");

    // Write-through: the global store lists it immediately.
    let listed = project::project_list(&services).expect("list projects");
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].project_id, created.project_id);

    let fetched = project::project_get(&services, &created.project_id).expect("get project");
    assert_eq!(fetched.name, "Andromeda Test");
}

#[test]
fn duplicate_create_fails() {
    let (services, _keep) = temp_services();

    project::project_create(&services, "Dup Target", None, "0.1.0").expect("first create");
    let second = project::project_create(&services, "Dup Target", None, "0.1.0");
    assert!(second.is_err(), "duplicate slug must be rejected");
}
