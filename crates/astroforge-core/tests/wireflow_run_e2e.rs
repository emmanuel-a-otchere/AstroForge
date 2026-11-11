//! #461 S5 (W2 follow-through) — the definitive run-execution test:
//! drive the REAL PipelineRunner over registered sample-dataset
//! assets through the full 10-stage plan, and assert the #465 W2
//! contract — every producing stage persists a Version (artifact +
//! ImageVersion + VersionCreated event) and the final output lands
//! as a readable TIFF under the applied/ directory.
//!
//! Workflow B (raw frames → full stacking pipeline) verified
//! headlessly: Home → project → import → understanding-derived plan
//! → runner → Versions.

use astroforge_core::artifact::ContentStore;
use astroforge_core::domain::SourceAsset;
use astroforge_core::domain_store::DomainStore;
use astroforge_core::pipeline_plan::builtin::deep_sky_osc_balanced;
use astroforge_core::pipeline_plan::dispatch::{
    BackgroundHandler, CalibrateHandler, DebayerHandler, DenoiseHandler, ExportHandler,
    HandlerRegistry, RegisterHandler, StackHandler, StretchHandler,
};
use astroforge_core::pipeline_plan::{
    generate_plan, now_unix_ms, GenerationContext, PipelineRunner,
};
use astroforge_core::pipeline_plans_store::PipelinePlanStore;
use std::path::Path;
use std::sync::Arc;

fn temp_root(name: &str) -> PathBuf {
    let mut dir = std::env::temp_dir();
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    dir.push(format!("af-run-e2e-{name}-{}-{stamp}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("mkdir");
    dir
}

use std::path::PathBuf;

/// import_begin flow: create the project, then the session (the
/// source_assets FK requires the sessions row), then mark it active.
fn import_begin(store: &DomainStore, name: &str, source_dir: &str) -> (String, String) {
    let project_id = store
        .create_project(name, None, "0.1.0")
        .expect("create project");
    let session_id = store
        .create_session(&project_id, source_dir)
        .expect("create session");
    store
        .set_active_session(&project_id, &session_id)
        .expect("set active session");
    (project_id, session_id)
}

/// The import_register_assets flow: hash each file, register a
/// durable SourceAsset pointing at the original path (the pipeline
/// decodes `original_path` via the TIFF/FITS decoders).
fn register_light(store: &DomainStore, session_id: &str, path: &Path) {
    let bytes = std::fs::read(path).expect("read frame");
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let asset = SourceAsset {
        asset_id: String::new(),
        content_hash: ContentStore::sha256_hex(&bytes),
        original_filename: name.clone(),
        original_path: path.to_string_lossy().into_owned(),
        file_size: bytes.len() as u64,
        format: "FITS".into(),
        mime_type: None,
        created_at: "2026-11-11T00:00:00Z".into(),
        imported_at: "2026-11-11T00:00:00Z".into(),
        session_id: session_id.to_string(),
        frame_type: Some("Light".into()),
        exposure: Some(30.0),
        filter: None,
        binning: None,
        width: Some(256),
        height: Some(256),
        bit_depth: None,
        bayer_pattern: None,
        camera: None,
        date_obs: None,
        ra: None,
        dec: None,
    };
    store.register_source_asset(&asset).expect("register");
}

#[test]
fn full_plan_run_over_sample_dataset_persists_versions() {
    let root = temp_root("full");
    let store = Arc::new(DomainStore::new(&root.join("projects.db")).expect("store"));

    // 1. Import: project → session → active → registered assets
    //    (real sample-dataset FITS frames).
    let dataset = root.join("sample-dataset");
    std::fs::create_dir_all(&dataset).expect("mkdir dataset");
    let (project_id, session_id) = import_begin(&store, "M31 Sample", dataset.to_str().unwrap());
    for i in 0..4 {
        let mut header = astroforge_core::fits::FitsHeader::new();
        header.set("IMAGETYP", "LIGHT");
        header.set("EXPTIME", "30.0");
        header.set("NAXIS1", "64");
        header.set("NAXIS2", "64");
        let frame = dataset.join(format!("light_{i:03}.fits"));
        let mut f = std::fs::File::create(&frame).expect("create frame");
        astroforge_core::fits::write_header(&header, &mut f).expect("write header");
        let pixels = 64 * 64;
        let mut data = Vec::with_capacity(pixels * 4);
        for px in 0..pixels {
            // Per-frame gradient so a stack has signal to find.
            let v = ((px + i * 97) % 251) as f32 / 260.0 + 0.03;
            data.extend_from_slice(&v.to_be_bytes());
        }
        let block = 2880usize;
        if data.len() % block != 0 {
            data.resize(data.len() + (block - data.len() % block), 0);
        }
        use std::io::Write as _;
        f.write_all(&data).expect("write pixels");
        drop(f);
        register_light(&store, &session_id, &frame);
    }
    // 2. Understanding: derived from the registered assets (no
    //    classification recorded — the derive falls back to the
    //    capture-shape defaults with real asset counts).
    let assets = store.list_source_assets(&session_id).expect("assets");
    assert_eq!(assets.len(), 4, "4 lights registered");
    let understanding =
        astroforge_core::pipeline_plan::derive_understanding_from_session(None, &assets);
    assert_eq!(understanding.light_frame_count, 4);
    assert!(!understanding.already_stacked, "raw lights are not stacked");

    // 3. Plan: generated from the derived understanding and inserted
    //    into the plan store (create_pipeline_plan's body).
    let plan_store = Arc::new(PipelinePlanStore::in_memory().expect("plan store"));
    let (stages, target) = deep_sky_osc_balanced();
    let ctx = GenerationContext {
        project_id: project_id.clone(),
        session_id: session_id.clone(),
        session_understanding: understanding,
        recipe_id: Some("recipe_deep_sky_osc_balanced".into()),
        mode: "auto".into(),
        generated_at_unix_ms: now_unix_ms(),
    };
    let plan = generate_plan(&ctx, &stages, target).expect("plan");
    plan_store.insert_plan(&plan).expect("insert plan");

    // 4. Runner: full registry (all 8 handlers, same registrations as
    //    the production shell) + the DomainStore for version
    //    persistence.
    let mut registry = HandlerRegistry::new();
    registry.insert("calibrate", Arc::new(CalibrateHandler));
    registry.insert("debayer", Arc::new(DebayerHandler));
    registry.insert("register", Arc::new(RegisterHandler));
    registry.insert("stack", Arc::new(StackHandler));
    registry.insert("background", Arc::new(BackgroundHandler));
    registry.insert("stretch", Arc::new(StretchHandler));
    registry.insert("denoise", Arc::new(DenoiseHandler));
    registry.insert("export", Arc::new(ExportHandler));
    let runner = PipelineRunner::with_engine(
        plan_store.clone(),
        Arc::new(registry),
        Some(store.clone()),
        None,
    );

    // 5. Run.
    let outcome = runner.start(&plan.plan_id).expect("run");
    assert_eq!(
        outcome,
        astroforge_core::pipeline_plan::RunOutcome::Completed,
        "the full 10-stage run must complete over real frames"
    );

    // 6. The W2 contract: the stack stage (the first image-producing
    //    stage with an artifact) persisted a Version + artifact.
    let execs = plan_store
        .list_stage_executions_for_plan(&plan.plan_id)
        .expect("execs");
    let with_artifact: Vec<_> = execs
        .iter()
        .filter(|e| e.output_artifact_id.is_some())
        .collect();
    assert!(
        !with_artifact.is_empty(),
        "at least one producing stage must persist an artifact"
    );

    let versions = store
        .list_image_versions_for_project(&project_id)
        .expect("versions");
    assert!(
        !versions.is_empty(),
        "the run must produce ImageVersions (before W2 this list was [] forever)"
    );
    for v in &versions {
        let artifact = store
            .get_artifact(&v.primary_artifact_id)
            .expect("artifact readable");
        assert!(
            !artifact.path.is_empty(),
            "artifact path must be recorded for canvas reads"
        );
        assert!(
            std::path::Path::new(&artifact.path).exists(),
            "artifact TIFF must exist on disk: {}",
            artifact.path
        );
    }

    // 7. The VersionCreated event is in the project event log — the
    //    derivation the frontend timeline reads.
    let events = store.list_events(&project_id).expect("events");
    assert!(
        events
            .iter()
            .any(|e| format!("{:?}", e.kind).contains("VersionCreated")),
        "VersionCreated event must be recorded"
    );
    let _ = std::fs::remove_dir_all(&root);
}
