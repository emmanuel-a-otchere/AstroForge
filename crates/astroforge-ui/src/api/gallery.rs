//! Gallery CRUD — port of the inline gallery commands in
//! `src-tauri/src/main.rs` (`gallery_list` / `gallery_upsert` /
//! `gallery_delete`).
//!
//! Each function is a thin wrapper over `GalleryStore` with bodies
//! ported 1:1 from the corresponding Tauri command; errors are
//! `String` with the same text the Tauri commands surfaced.

use crate::services::Services;

use astroforge_core::gallery::{GalleryItem, GalleryItemUpdate};

fn lock_err<E: std::fmt::Display>(e: E) -> String {
    format!("lock poisoned: {e}")
}

/// `gallery_list` — every item in the gallery store.
pub fn gallery_list(services: &Services) -> Result<Vec<GalleryItem>, String> {
    let store = services.gallery.lock().map_err(lock_err)?;
    store.list().map_err(|e| e.to_string())
}

/// `gallery_upsert` — insert-or-update one gallery item from the
/// caller-supplied `GalleryItemUpdate`.
pub fn gallery_upsert(
    services: &Services,
    update: GalleryItemUpdate,
) -> Result<GalleryItem, String> {
    let store = services.gallery.lock().map_err(lock_err)?;
    store.upsert(update).map_err(|e| e.to_string())
}

/// `gallery_delete` — remove one gallery item by id.
pub fn gallery_delete(services: &Services, id: &str) -> Result<(), String> {
    let store = services.gallery.lock().map_err(lock_err)?;
    store.delete(id).map_err(|e| e.to_string())
}
