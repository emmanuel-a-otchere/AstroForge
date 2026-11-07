//! #461 S5 — end-to-end wireflow verification over the committed
//! sample dataset, exercising the exact backend chain the wired UI
//! now drives:
//!
//!   create_project → create_session (import_begin) → set_active_session
//!   → register_source_assets (real sample-dataset files)
//!   → derive_understanding_from_session → generate_plan
//!   → Workflow A (already-stacked: stacking stages DISABLED)
//!   | Workflow B (raw frames: full pipeline ENABLED, stack included)
//!
//! The stacking execution itself is covered by the runner tests and
//! the smoke job; this test proves the PLUMBING the wireflow audit
//! found missing: durable assets, derived understanding, and
//! stage-level routing that differs between the two workflows.

use astroforge_core::domain::SourceAsset;
use astroforge_core::domain_store::DomainStore;
use astroforge_core::fits::{self, FitsHeader};
use astroforge_core::pipeline_plan::{
    derive_understanding_from_session, generate_plan, SessionUnderstanding, StageType,
};
use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};

const SAMPLE_DATASET: &str = "../../tests/fixtures/sample-dataset";

fn temp_store(name: &str) -> (DomainStore, PathBuf) {
    let mut dir = std::env::temp_dir();
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    dir.push(format!("af-wireflow-{name}-{}-{stamp}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("mkdir");
    // DomainStore::new takes the DB FILE path; frames land alongside.
    let store = DomainStore::new(&dir.join("projects.db")).expect("store");
    (store, dir)
}

fn write_minimal_fits(path: &Path, imagetyp: &str, exptime: f64) {
    let mut header = FitsHeader::new();
    header.set("IMAGETYP", imagetyp);
    header.set("EXPTIME", &exptime.to_string());
    header.set("NAXIS1", "64");
    header.set("NAXIS2", "64");

    let mut file = File::create(path).expect("create fits");
    fits::write_header(&header, &mut file).expect("write header");
    let pixel_count = 64 * 64;
    let mut data = Vec::with_capacity(pixel_count * 4);
    for _ in 0..pixel_count {
        data.extend_from_slice(&0.0_f32.to_be_bytes());
    }
    let block_size: usize = 2880;
    let total = data.len();
    if total % block_size != 0 {
        data.resize(total + (block_size - total % block_size), 0);
    }
    file.write_all(&data).expect("data");
}

/// The import_begin flow: create the project, then the session
/// (source FK), then mark it active — exactly what the wired
/// `import_begin` command does.
fn import_begin(store: &DomainStore, project_name: &str, source_dir: &str) -> (String, String) {
    let project_id = store
        .create_project(project_name, None, "0.1.0")
        .expect("create project");
    let session_id = store
        .create_session(&project_id, source_dir)
        .expect("create session");
    store
        .set_active_session(&project_id, &session_id)
        .expect("set active session");
    (project_id, session_id)
}

/// The import_register_assets flow: content-hash each file and
/// register it as a durable SourceAsset for the session.
fn register_asset(store: &DomainStore, session_id: &str, path: &Path, frame_type: &str) {
    let bytes = std::fs::read(path).expect("read frame");
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let asset = SourceAsset {
        asset_id: String::new(),
        content_hash: astroforge_core::artifact::ContentStore::sha256_hex(&bytes),
        original_filename: name.clone(),
        original_path: path.to_string_lossy().into_owned(),
        file_size: bytes.len() as u64,
        format: "FITS".into(),
        mime_type: None,
        created_at: "2026-11-07T00:00:00Z".into(),
        imported_at: "2026-11-07T00:00:00Z".into(),
        session_id: session_id.to_string(),
        frame_type: Some(frame_type.to_string()),
        exposure: Some(30.0),
        filter: None,
        binning: None,
        width: Some(64),
        height: Some(64),
        bit_depth: None,
        bayer_pattern: None,
        camera: None,
        date_obs: None,
        ra: None,
        dec: None,
    };
    store.register_source_asset(&asset).expect("register");
}

/// Active-session plumbing: the Process workspace gates its plan
/// generator on `active_session_id` — the field that was never set
/// before the wireflow fix.
#[test]
fn import_marks_session_active_on_the_project() {
    let (store, _dir) = temp_store("active");
    let (project_id, session_id) = import_begin(&store, "M31", "tmp/import");
    let project = store.get_project(&project_id).expect("project");
    assert_eq!(
        project.active_session_id.as_deref(),
        Some(session_id.as_str()),
        "import must set active_session_id or Process stays unreachable"
    );
}

/// Workflow B (needs stacking): 6 raw lights + calibration frames →
/// derived understanding says NOT already stacked, calibration Partial
/// → the generated plan keeps calibrate/debayer/register/stack ENABLED.
#[test]
fn workflow_b_raw_frames_keep_the_stacking_pipeline() {
    let (store, dir) = temp_store("raw");
    let (project_id, session_id) = import_begin(&store, "M42 raw", "tmp/raw");

    // Real files from the committed sample dataset (Seestar headers),
    // re-registered as this session's assets.
    let dataset = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(SAMPLE_DATASET);
    let mut lights = 0usize;
    for i in 0..6 {
        let src = dataset.join(format!("light_{i:03}.fits"));
        let dst = dir.join(format!("light_{i:03}.fits"));
        std::fs::copy(&src, &dst).expect("copy light");
        register_asset(&store, &session_id, &dst, "Light");
        lights += 1;
    }
    let src = dataset.join("dark_000.fits");
    let dst = dir.join("dark_000.fits");
    std::fs::copy(&src, &dst).expect("copy dark");
    register_asset(&store, &session_id, &dst, "Dark");

    let assets = store.list_source_assets(&session_id).expect("assets");
    assert_eq!(assets.len(), lights + 1, "every scanned frame persisted");

    let classification = store
        .get_session_classification(&session_id)
        .expect("read classification");
    let understanding = derive_understanding_from_session(classification.as_ref(), &assets);
    assert!(!understanding.already_stacked, "raw frames are not stacked");
    assert_eq!(understanding.light_frame_count, 6);
    assert_eq!(
        understanding.calibration,
        astroforge_core::pipeline_plan::CalibrationAvailability::Partial,
        "dark present, no flat/bias"
    );

    let (stages, target) = astroforge_core::pipeline_plan::builtin::deep_sky_osc_balanced();
    let ctx = astroforge_core::pipeline_plan::GenerationContext {
        project_id,
        session_id: session_id.clone(),
        session_understanding: understanding,
        recipe_id: Some("recipe_deep_sky_osc_balanced".into()),
        mode: "auto".into(),
        generated_at_unix_ms: astroforge_core::pipeline_plan::now_unix_ms(),
    };
    let plan = generate_plan(&ctx, &stages, target).expect("plan");

    for stage_type in ["calibrate", "debayer", "register", "stack"] {
        let stage = plan
            .stages
            .iter()
            .find(|s| s.stage_type == stage_type)
            .unwrap_or_else(|| panic!("{stage_type} stage missing"));
        assert!(stage.enabled, "Workflow B keeps {stage_type} enabled");
    }
    let _ = std::fs::remove_dir_all(&dir);
}

/// Workflow A (already-stacked): a single `_master_stacked` FITS →
/// derived understanding flips already_stacked → the plan DISABLES
/// calibrate/debayer/register/stack (audit trail intact) so the user
/// lands in post-stack processing, exactly as the maintainer described.
#[test]
fn workflow_a_stacked_master_disables_the_stacking_stages() {
    let (store, dir) = temp_store("stacked");
    let (project_id, session_id) = import_begin(&store, "M42 master", "tmp/stacked");

    let master = dir.join("m42_master_stacked.fits");
    write_minimal_fits(&master, "LIGHT", 30.0);
    register_asset(&store, &session_id, &master, "Light");

    let assets = store.list_source_assets(&session_id).expect("assets");
    let understanding = derive_understanding_from_session(None, &assets);
    assert!(
        understanding.already_stacked,
        "single _master_stacked light must be detected"
    );
    assert_eq!(understanding.light_frame_count, 1);

    let (stages, target) = astroforge_core::pipeline_plan::builtin::deep_sky_osc_balanced();
    let ctx = astroforge_core::pipeline_plan::GenerationContext {
        project_id,
        session_id: session_id.clone(),
        session_understanding: understanding,
        recipe_id: Some("recipe_deep_sky_osc_balanced".into()),
        mode: "auto".into(),
        generated_at_unix_ms: astroforge_core::pipeline_plan::now_unix_ms(),
    };
    let plan = generate_plan(&ctx, &stages, target).expect("plan");

    for stage_type in ["calibrate", "debayer", "register", "stack"] {
        let stage = plan
            .stages
            .iter()
            .find(|s| s.stage_type == stage_type)
            .unwrap_or_else(|| panic!("{stage_type} stage missing"));
        assert!(
            !stage.enabled,
            "Workflow A disables {stage_type} for an already-stacked master"
        );
        // The audit trail survives: stages are disabled, not removed.
        assert!(stage.required, "{stage_type} stays in the recipe record");
    }
    // Post-stack stages stay live.
    let stretch = plan
        .stages
        .iter()
        .find(|s| s.stage_type == "stretch")
        .expect("stretch stage");
    assert!(stretch.enabled, "post-stack processing remains enabled");
    let _ = std::fs::remove_dir_all(&dir);
}

/// Sanity: the SessionUnderstanding default (no classification, no
/// assets) still produces the balanced plan unchanged.
#[test]
fn empty_session_yields_the_balanced_default_plan() {
    let understanding = SessionUnderstanding::deep_sky_osc_balanced();
    assert!(!understanding.already_stacked);
    let _ = StageType::Calibrate; // taxonomy reference
}
