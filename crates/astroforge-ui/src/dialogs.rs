//! Dialogs + native file-picker helpers (migration P2 chrome).
//!
//! `confirm_modal` is the single destructive-action gate: every
//! irreversible action in the app must route through it (DESIGN.md
//! Level-2 overlay semantics: scrim + elevated surface + error accent
//! for dangerous confirmations). Native open/save dialogs go through
//! `rfd`'s blocking API — acceptable for a desktop app whose dialogs
//! are momentary; long-running work must never wait on them.

use egui::{Frame, Margin, RichText, Stroke, Ui};

use crate::tokens as t;

/// A pending confirmation. `action` is an opaque payload the shell
/// interprets when the user confirms (keeps this module UI-only).
#[derive(Clone, Debug)]
pub struct Confirm {
    pub id: egui::Id,
    pub title: String,
    pub message: String,
    pub confirm_label: String,
    pub danger: bool,
}

impl Confirm {
    pub fn new(title: &str, message: &str, confirm_label: &str, danger: bool) -> Self {
        Self {
            // Id::new is the only stable salted Id constructor; the
            // 0.37 deprecation points at AsIdSalt, which Modal::new
            // does not accept (it takes a raw Id).
            #[allow(deprecated)]
            id: egui::Id::new("confirm").with(title).with(message),
            title: title.to_string(),
            message: message.to_string(),
            confirm_label: confirm_label.to_string(),
            danger,
        }
    }
}

/// What the user decided this frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConfirmOutcome {
    /// Keep showing.
    Pending,
    /// User pressed the confirm button.
    Confirmed,
    /// Backdrop click / Esc / Cancel.
    Dismissed,
}

/// Draw the confirmation modal over the whole window. Call once per
/// frame while a confirmation is pending.
pub fn confirm_modal(ctx: &egui::Context, confirm: &Confirm) -> ConfirmOutcome {
    let mut outcome = ConfirmOutcome::Pending;

    let modal = egui::Modal::new(confirm.id)
        .backdrop_color(t::elevation::SCRIM)
        .frame(
            Frame::default()
                .fill(t::surface::CONTAINER)
                .stroke(Stroke::new(
                    1.0,
                    if confirm.danger {
                        t::Status::Error.border().color
                    } else {
                        t::border::HAIRLINE
                    },
                ))
                .corner_radius(t::radius::LG)
                .inner_margin(Margin::same(t::spacing::LG as i8)),
        );

    let response = modal.show(ctx, |ui| {
        ui.set_min_width(360.0);
        ui.label(
            RichText::new(&confirm.title)
                .size(t::type_scale::HEADLINE.size * 0.7)
                .family(t::type_scale::HEADLINE.family.clone())
                .color(if confirm.danger {
                    t::Status::Error.fg()
                } else {
                    t::text::PRIMARY
                }),
        );
        ui.add_space(t::spacing::SM);
        ui.label(
            RichText::new(&confirm.message)
                .size(t::type_scale::BODY.size)
                .color(t::text::SECONDARY),
        );
        ui.add_space(t::spacing::LG);
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let confirm_clicked = ui
                .button(
                    RichText::new(&confirm.confirm_label).color(if confirm.danger {
                        t::Status::Error.fg()
                    } else {
                        t::accent::PRIMARY
                    }),
                )
                .clicked();
            let _ = ui.button("Cancel");
            confirm_clicked
        })
        .inner
    });

    if response.inner {
        outcome = ConfirmOutcome::Confirmed;
    } else if response.should_close() {
        outcome = ConfirmOutcome::Dismissed;
    }

    outcome
}

// ─── Native file dialogs (rfd) ────────────────────────────────────────────

/// Pick a folder (e.g. FITS source directory for import).
pub fn pick_directory(title: &str) -> Option<std::path::PathBuf> {
    rfd::FileDialog::new().set_title(title).pick_folder()
}

/// Pick an existing file (e.g. recipe JSON import).
pub fn pick_open_file(title: &str, extensions: &[&str]) -> Option<std::path::PathBuf> {
    rfd::FileDialog::new()
        .set_title(title)
        .add_filter("Supported", extensions)
        .pick_file()
}

/// Pick a save target (e.g. recipe JSON export).
pub fn pick_save_file(
    title: &str,
    file_name: &str,
    extensions: &[&str],
) -> Option<std::path::PathBuf> {
    rfd::FileDialog::new()
        .set_title(title)
        .set_file_name(file_name)
        .add_filter("Supported", extensions)
        .save_file()
}

/// Drag-and-drop intake: files dropped onto the window this frame.
/// The Import workspace consumes these; other screens ignore them.
pub fn dropped_files(ctx: &egui::Context) -> Vec<std::path::PathBuf> {
    ctx.input(|i| {
        i.raw
            .dropped_files
            .iter()
            .map(|f| f.path().to_path_buf())
            .collect()
    })
}

/// Read a small text file with a friendly error path (recipe
/// import/export share this).
pub fn read_text(path: &std::path::Path) -> Result<String, String> {
    std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))
}

pub fn write_text(path: &std::path::Path, contents: &str) -> Result<(), String> {
    std::fs::write(path, contents).map_err(|e| format!("{}: {e}", path.display()))
}

/// Small helper for screens: an inline "browse..." button + chosen
/// path readout.
pub fn path_picker(ui: &mut Ui, label: &str, chosen: &mut Option<std::path::PathBuf>) -> bool {
    let mut picked = false;
    ui.horizontal(|ui| {
        if ui.button(label).clicked() {
            picked = true;
        }
        if let Some(p) = chosen {
            ui.label(
                RichText::new(p.display().to_string())
                    .size(t::type_scale::METADATA.size)
                    .color(t::text::SECONDARY),
            );
        }
    });
    picked
}
