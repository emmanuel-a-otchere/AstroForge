use astroforge_core::gallery::{GalleryItemUpdate, GalleryStatus, GalleryStore};
use std::path::PathBuf;

fn tmp_db_path(name: &str) -> PathBuf {
    let mut p = std::env::temp_dir();
    p.push(format!(
        "astroforge-gallery-{}-{}.sqlite",
        name,
        std::process::id()
    ));
    // Ensure unique name across runs
    p.set_extension(format!(
        "sqlite.{}.{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos(),
        std::process::id(),
    ));
    let _ = std::fs::remove_file(&p);
    p
}

#[test]
fn gallery_seeds_on_first_open() {
    let path = tmp_db_path("seed");
    let store = GalleryStore::new(&path).expect("open");
    let items = store.list().expect("list");
    assert_eq!(items.len(), 5, "expected 5 placeholder seeds");
    let m42 = items
        .iter()
        .find(|i| i.id == "placeholder-m42")
        .expect("M42 seed");
    assert_eq!(m42.target, "M42");
    assert!((m42.integration_hours - 7.36).abs() < 0.01);
    assert_eq!(m42.status, GalleryStatus::Completed);
}

#[test]
fn gallery_seeds_only_once() {
    let path = tmp_db_path("seed-once");
    let store = GalleryStore::new(&path).expect("open first");
    let _ = store.list().expect("list first");
    drop(store);
    let store = GalleryStore::new(&path).expect("open second");
    let items = store.list().expect("list second");
    assert_eq!(items.len(), 5, "second open should not re-seed");
}

#[test]
fn gallery_upsert_inserts_new_with_minted_id() {
    let path = tmp_db_path("upsert-new");
    let store = GalleryStore::new(&path).expect("open");
    let initial = store.list().expect("list").len();

    let item = store
        .upsert(GalleryItemUpdate {
            id: None,
            name: "Orion_TEST_RUN".into(),
            target: "M42".into(),
            integration_hours: 2.5,
            palette: "LRGB".into(),
            status: GalleryStatus::Processing,
        })
        .expect("upsert");

    assert!(
        item.id.starts_with("gallery_"),
        "minted id should be prefixed"
    );
    assert_eq!(item.name, "Orion_TEST_RUN");
    assert_eq!(item.status, GalleryStatus::Processing);
    assert_eq!(store.list().expect("list").len(), initial + 1);
}

#[test]
fn gallery_upsert_updates_existing() {
    let path = tmp_db_path("upsert-update");
    let store = GalleryStore::new(&path).expect("open");
    let initial = store.list().expect("list").len();

    // Re-upsert with the M42 placeholder id; this should update in place.
    let item = store
        .upsert(GalleryItemUpdate {
            id: Some("placeholder-m42".into()),
            name: "M42_REPROCESSED".into(),
            target: "M42".into(),
            integration_hours: 12.0,
            palette: "HSO".into(),
            status: GalleryStatus::Completed,
        })
        .expect("upsert");

    assert_eq!(item.id, "placeholder-m42");
    assert_eq!(item.name, "M42_REPROCESSED");
    assert!((item.integration_hours - 12.0).abs() < 0.001);
    assert_eq!(item.palette, "HSO");
    assert_eq!(item.status, GalleryStatus::Completed);
    assert_eq!(
        store.list().expect("list").len(),
        initial,
        "no new row inserted"
    );
}

#[test]
fn gallery_delete_removes_row() {
    let path = tmp_db_path("delete");
    let store = GalleryStore::new(&path).expect("open");
    let initial = store.list().expect("list").len();

    store.delete("placeholder-m31").expect("delete");

    assert_eq!(store.list().expect("list").len(), initial - 1);
    assert!(store
        .list()
        .expect("list")
        .iter()
        .all(|i| i.id != "placeholder-m31"));
}

#[test]
fn gallery_delete_missing_id_is_noop() {
    let path = tmp_db_path("delete-missing");
    let store = GalleryStore::new(&path).expect("open");
    let initial = store.list().expect("list").len();
    // Should not error on missing id.
    store.delete("does-not-exist").expect("delete missing");
    assert_eq!(store.list().expect("list").len(), initial);
}

#[test]
fn gallery_status_roundtrip_string() {
    assert_eq!(GalleryStatus::Pending.as_str(), "pending");
    assert_eq!(GalleryStatus::Processing.as_str(), "processing");
    assert_eq!(GalleryStatus::Completed.as_str(), "completed");

    assert_eq!(
        GalleryStatus::parse("pending"),
        Some(GalleryStatus::Pending)
    );
    assert_eq!(
        GalleryStatus::parse("processing"),
        Some(GalleryStatus::Processing)
    );
    assert_eq!(
        GalleryStatus::parse("completed"),
        Some(GalleryStatus::Completed)
    );
    assert_eq!(GalleryStatus::parse("nonsense"), None);
}

#[test]
fn gallery_list_orders_by_updated_at_desc() {
    let path = tmp_db_path("order");
    let store = GalleryStore::new(&path).expect("open");
    // Drop seeded rows so order is deterministic from our inserts.
    for id in [
        "placeholder-m42",
        "placeholder-m31",
        "placeholder-ngc7000",
        "placeholder-ic1396",
        "placeholder-horsehead",
    ] {
        store.delete(id).expect("delete seed");
    }

    store
        .upsert(GalleryItemUpdate {
            id: Some("first".into()),
            name: "First".into(),
            target: "X".into(),
            integration_hours: 1.0,
            palette: "LRGB".into(),
            status: GalleryStatus::Pending,
        })
        .expect("first");

    // Tiny sleep to ensure a distinct updated_at (datetime('now') has
    // 1-second resolution).
    std::thread::sleep(std::time::Duration::from_millis(1100));

    store
        .upsert(GalleryItemUpdate {
            id: Some("second".into()),
            name: "Second".into(),
            target: "X".into(),
            integration_hours: 2.0,
            palette: "LRGB".into(),
            status: GalleryStatus::Pending,
        })
        .expect("second");

    let items = store.list().expect("list");
    assert_eq!(items.len(), 2);
    assert_eq!(items[0].id, "second", "most-recent should sort first");
    assert_eq!(items[1].id, "first");
}
