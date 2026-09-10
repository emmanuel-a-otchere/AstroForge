//! CR-06 P2 — image analysis integration tests.
//!
//! The analyzer is a pure function on `F32Image`. These
//! tests cover the report shape + the round-trip
//! JSON serialization that the IPC command depends on.

use astroforge_core::domain::ImageAnalysis;
use astroforge_core::domain_store::DomainStore;
use astroforge_core::image::F32Image;
use astroforge_core::image_analysis::report::{analyze_at, ImageAnalysisReport};
use std::path::PathBuf;

fn store() -> DomainStore {
    DomainStore::new(&PathBuf::from(":memory:")).expect("open in-memory store")
}

#[test]
fn analyze_round_trips_through_json() {
    let mut img = F32Image::new(256, 256, 1);
    // 96×96 blob to clear the bright-core detector.
    for y in 80..176 {
        for x in 80..176 {
            img[(0usize, y, x)] = 1.0;
        }
    }
    let report = analyze_at(&img, "ver_e2e", "test-0.1.0", "2026-09-10T00:00:00Z");
    let json = report.to_json().expect("to_json");
    let back: ImageAnalysisReport = ImageAnalysisReport::from_json(&json).expect("from_json");
    assert_eq!(report, back);
    assert!(back.has_bright_core);
    assert_eq!(back.image_version_id, "ver_e2e");
    assert_eq!(back.width, 256);
    assert_eq!(back.height, 256);
}

#[test]
fn analyze_then_persist_to_store() {
    // The IPC `analyze_image` command's persistence path
    // is `upsert_image_analysis(report)`. Pin the contract
    // by running the same calls against an in-memory
    // store.
    let s = store();
    let img = F32Image::new(64, 64, 1);
    let report = analyze_at(&img, "ver_persist", "test-0.1.0", "2026-09-10T00:00:00Z");
    let profile_json = report.to_json().expect("to_json");
    let row = ImageAnalysis {
        analysis_id: "ana_e2e_1".into(),
        project_id: String::new(),
        image_version_id: report.image_version_id.clone(),
        profile_json: profile_json.clone(),
        created_at: report.created_at.clone(),
    };
    s.upsert_image_analysis(&row).expect("upsert");
    let back = s
        .latest_image_analysis_for_version("ver_persist")
        .expect("read")
        .expect("present");
    assert_eq!(back.profile_json, profile_json);
    // The round-tripped JSON still parses as an
    // `ImageAnalysisReport`.
    let back_report: ImageAnalysisReport =
        ImageAnalysisReport::from_json(&back.profile_json).expect("reparse");
    assert_eq!(back_report.image_version_id, "ver_persist");
}

#[test]
fn analyze_persists_only_latest_per_version() {
    let s = store();
    for tag in ["first", "second", "third"] {
        let img = F32Image::new(32, 32, 1);
        let report = analyze_at(&img, "ver_latest", "test-0.1.0", "2026-09-10T00:00:00Z");
        let row = ImageAnalysis {
            analysis_id: format!("ana_{tag}"),
            project_id: String::new(),
            image_version_id: "ver_latest".into(),
            profile_json: report.to_json().expect("to_json"),
            created_at: format!(
                "2026-09-10T00:00:0{tag}Z",
                tag = match tag {
                    "first" => "1",
                    "second" => "2",
                    "third" => "3",
                    _ => "0",
                }
            ),
        };
        s.upsert_image_analysis(&row).expect("upsert");
    }
    let back = s
        .latest_image_analysis_for_version("ver_latest")
        .expect("read")
        .expect("present");
    // ORDER BY created_at DESC tie-breaks on analysis_id;
    // the last-inserted row carries the highest id, so
    // it's the latest.
    assert_eq!(back.analysis_id, "ana_third");
}
