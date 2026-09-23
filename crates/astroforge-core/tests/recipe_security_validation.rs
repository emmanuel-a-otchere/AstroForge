//! CR-08 §20: Recipe security validation tests.
//!
//! Pinning the five-check contract:
//! - Range (param numeric bounds)
//! - Dependency (stage required_stages present + enabled)
//! - Filesystem reference rejection
//! - Executable payload rejection
//! - Resource budget ceiling
//!
//! Plus composition: a Recipe that fails all five
//! classes at once surfaces every violation in a
//! single pass (rather than one at a time).
//!
//! Coverage:
//! - safe Recipe returns an empty violations list
//! - out-of-range numeric param produces a `range`
//!   violation with stage_id + param_key
//! - missing required stage produces a `dependency`
//!   violation
//! - spec-catalog absence produces a `dependency`
//!   violation
//! - `/abs/path` string param produces a
//!   `filesystem` violation
//! - `~/relative` string param produces a
//!   `filesystem` violation
//! - URL scheme (`://`) string param produces a
//!   `filesystem` violation
//! - shebang `#!/` string param produces an
//!   `executable` violation
//! - `os.system(` / `subprocess.` / `eval(` /
//!   `<script` / `</script` / `exec(` / `shell_exec`
//!   string params produce an `executable`
//!   violation
//! - resource budget over MAX_RESOURCE_UNITS
//!   produces a `resource` violation
//! - legitimate-looking Recipe (no violations)
//!   passes
//! - composition test: a single Recipe that
//!   fails all five classes at once surfaces
//!   every violation in a single pass
//! - serde round-trip on the report

use astroforge_core::recipe::{QualityTargets, Recipe, RecipeStage};
use astroforge_core::validation::{
    recipe_level_targets_ranges, validate_recipe_security, ParamRange, SecurityValidationReport,
    SecurityViolation, StageSpec, MAX_RESOURCE_UNITS, RECIPE_LEVEL_STAGE_ID,
};
use serde_json::json;
use std::collections::HashMap;

fn recipe_with_stages(stages: Vec<(&str, bool, HashMap<String, serde_json::Value>)>) -> Recipe {
    let mut r = Recipe::new("test", "stretch");
    for (id, enabled, params) in stages {
        r.stages.push(RecipeStage {
            stage_id: id.into(),
            enabled,
            params,
            ai_enhancement_override: None,
        });
    }
    r
}

fn spec(id: &str, resource_units: u32) -> StageSpec {
    StageSpec::cheap(id, resource_units)
}

fn spec_with_range(id: &str, key: &str, range: ParamRange, resource_units: u32) -> StageSpec {
    let mut s = spec(id, resource_units);
    s.params.insert(key.into(), range);
    s
}

fn spec_with_required(id: &str, required_stages: &[&str], resource_units: u32) -> StageSpec {
    let mut s = spec(id, resource_units);
    s.required_stages = required_stages.iter().map(|s| s.to_string()).collect();
    s
}

fn find_kind<'a>(report: &'a SecurityValidationReport, kind: &str) -> Vec<&'a SecurityViolation> {
    report
        .violations
        .iter()
        .filter(|v| v.kind == kind)
        .collect()
}

#[test]
fn safe_recipe_returns_empty_violations() {
    let recipe = recipe_with_stages(vec![(
        "stretch",
        true,
        HashMap::from([("radius".into(), json!(3.0))]),
    )]);
    let mut specs = HashMap::new();
    specs.insert(
        "stretch".into(),
        spec_with_range("stretch", "radius", ParamRange::bounded(0.0, 10.0), 50),
    );
    let report = validate_recipe_security(&recipe, &specs);
    assert!(report.is_safe(), "safe Recipe must have empty violations");
    assert_eq!(report.violations.len(), 0);
    assert_eq!(report.total_resource_units, 50);
}

#[test]
fn out_of_range_param_produces_range_violation() {
    let recipe = recipe_with_stages(vec![(
        "stretch",
        true,
        HashMap::from([("radius".into(), json!(15.0))]),
    )]);
    let mut specs = HashMap::new();
    specs.insert(
        "stretch".into(),
        spec_with_range("stretch", "radius", ParamRange::bounded(0.0, 10.0), 50),
    );
    let report = validate_recipe_security(&recipe, &specs);
    assert!(!report.is_safe());
    let range_violations = find_kind(&report, "range");
    assert_eq!(range_violations.len(), 1);
    assert_eq!(range_violations[0].stage_id, Some("stretch".into()));
    assert_eq!(range_violations[0].param_key, Some("radius".into()));
}

#[test]
fn below_min_param_produces_range_violation() {
    let recipe = recipe_with_stages(vec![(
        "stretch",
        true,
        HashMap::from([("radius".into(), json!(-1.0))]),
    )]);
    let mut specs = HashMap::new();
    specs.insert(
        "stretch".into(),
        spec_with_range("stretch", "radius", ParamRange::bounded(0.0, 10.0), 50),
    );
    let report = validate_recipe_security(&recipe, &specs);
    let range_violations = find_kind(&report, "range");
    assert_eq!(range_violations.len(), 1);
    assert!(
        range_violations[0].message.contains("below the minimum"),
        "got message: {}",
        range_violations[0].message
    );
}

#[test]
fn above_max_param_produces_range_violation() {
    let recipe = recipe_with_stages(vec![(
        "stretch",
        true,
        HashMap::from([("radius".into(), json!(11.0))]),
    )]);
    let mut specs = HashMap::new();
    specs.insert(
        "stretch".into(),
        spec_with_range("stretch", "radius", ParamRange::bounded(0.0, 10.0), 50),
    );
    let report = validate_recipe_security(&recipe, &specs);
    let range_violations = find_kind(&report, "range");
    assert_eq!(range_violations.len(), 1);
    assert!(
        range_violations[0].message.contains("exceeds the maximum"),
        "got message: {}",
        range_violations[0].message
    );
}

#[test]
fn missing_required_stage_produces_dependency_violation() {
    // Stage A requires stage B, but B is missing.
    let recipe = recipe_with_stages(vec![("a", true, HashMap::new())]);
    let mut specs = HashMap::new();
    specs.insert("a".into(), spec_with_required("a", &["b"], 50));
    specs.insert("b".into(), spec("b", 50));
    let report = validate_recipe_security(&recipe, &specs);
    let dep_violations = find_kind(&report, "dependency");
    assert!(
        dep_violations
            .iter()
            .any(|v| { v.stage_id == Some("a".into()) && v.message.contains("requires 'b'") }),
        "expected dependency violation for missing 'b', got: {:?}",
        dep_violations
    );
}

#[test]
fn disabled_required_stage_produces_dependency_violation() {
    // Stage A requires stage B, but B is disabled.
    let recipe = recipe_with_stages(vec![
        ("a", true, HashMap::new()),
        ("b", false, HashMap::new()),
    ]);
    let mut specs = HashMap::new();
    specs.insert("a".into(), spec_with_required("a", &["b"], 50));
    specs.insert("b".into(), spec("b", 50));
    let report = validate_recipe_security(&recipe, &specs);
    let dep_violations = find_kind(&report, "dependency");
    assert!(
        dep_violations.iter().any(|v| {
            v.stage_id == Some("a".into())
                && v.message.contains("requires 'b'")
                && v.message.contains("missing or disabled")
        }),
        "expected dependency violation for disabled 'b', got: {:?}",
        dep_violations
    );
}

#[test]
fn enabled_required_stage_satisfies_dependency() {
    // Stage A requires stage B, and B is enabled.
    let recipe = recipe_with_stages(vec![
        ("a", true, HashMap::new()),
        ("b", true, HashMap::new()),
    ]);
    let mut specs = HashMap::new();
    specs.insert("a".into(), spec_with_required("a", &["b"], 50));
    specs.insert("b".into(), spec("b", 50));
    let report = validate_recipe_security(&recipe, &specs);
    // The dependency check is satisfied; no
    // dependency violations remain.
    let dep_violations = find_kind(&report, "dependency");
    assert_eq!(
        dep_violations.len(),
        0,
        "expected no dependency violations, got: {:?}",
        dep_violations
    );
}

#[test]
fn stage_with_no_spec_entry_produces_dependency_violation() {
    let recipe = recipe_with_stages(vec![("ghost", true, HashMap::new())]);
    let specs: HashMap<String, StageSpec> = HashMap::new();
    let report = validate_recipe_security(&recipe, &specs);
    let dep_violations = find_kind(&report, "dependency");
    assert!(
        dep_violations.iter().any(|v| {
            v.stage_id == Some("ghost".into()) && v.message.contains("not in the spec catalog")
        }),
        "expected dependency violation for spec-catalog absence, got: {:?}",
        dep_violations
    );
}

#[test]
fn absolute_posix_path_produces_filesystem_violation() {
    let recipe = recipe_with_stages(vec![(
        "stack",
        true,
        HashMap::from([("input_path".into(), json!("/tmp/raw.fits"))]),
    )]);
    let mut specs = HashMap::new();
    specs.insert("stack".into(), spec("stack", 50));
    let report = validate_recipe_security(&recipe, &specs);
    let fs_violations = find_kind(&report, "filesystem");
    assert_eq!(fs_violations.len(), 1);
    assert_eq!(fs_violations[0].param_key, Some("input_path".into()));
}

#[test]
fn home_relative_path_produces_filesystem_violation() {
    let recipe = recipe_with_stages(vec![(
        "stack",
        true,
        HashMap::from([("input_path".into(), json!("~/datasets/raw"))]),
    )]);
    let mut specs = HashMap::new();
    specs.insert("stack".into(), spec("stack", 50));
    let report = validate_recipe_security(&recipe, &specs);
    let fs_violations = find_kind(&report, "filesystem");
    assert_eq!(fs_violations.len(), 1);
}

#[test]
fn url_scheme_produces_filesystem_violation() {
    let recipe = recipe_with_stages(vec![(
        "stack",
        true,
        HashMap::from([("input_path".into(), json!("file:///etc/passwd"))]),
    )]);
    let mut specs = HashMap::new();
    specs.insert("stack".into(), spec("stack", 50));
    let report = validate_recipe_security(&recipe, &specs);
    let fs_violations = find_kind(&report, "filesystem");
    assert_eq!(fs_violations.len(), 1);
    assert!(
        fs_violations[0].message.contains("URL or path scheme"),
        "got message: {}",
        fs_violations[0].message
    );
}

#[test]
fn legitimate_relative_path_does_not_trigger_filesystem_violation() {
    // Relative paths are fine; the §20 rule
    // rejects absolute paths / home-relative /
    // URLs. A relative path like
    // "datasets/raw.fits" must pass.
    let recipe = recipe_with_stages(vec![(
        "stack",
        true,
        HashMap::from([("input_path".into(), json!("datasets/raw.fits"))]),
    )]);
    let mut specs = HashMap::new();
    specs.insert("stack".into(), spec("stack", 50));
    let report = validate_recipe_security(&recipe, &specs);
    let fs_violations = find_kind(&report, "filesystem");
    assert_eq!(fs_violations.len(), 0);
}

#[test]
fn shebang_produces_executable_violation() {
    let recipe = recipe_with_stages(vec![(
        "stack",
        true,
        HashMap::from([("script".into(), json!("#!/bin/bash\nls /"))]),
    )]);
    let mut specs = HashMap::new();
    specs.insert("stack".into(), spec("stack", 50));
    let report = validate_recipe_security(&recipe, &specs);
    let exec_violations = find_kind(&report, "executable");
    assert_eq!(exec_violations.len(), 1);
}

#[test]
fn python_signature_produces_executable_violation() {
    for needle in [
        "os.system(",
        "subprocess.run",
        "eval(",
        "<script>",
        "</script>",
        "exec(",
        "shell_exec",
    ] {
        let recipe = recipe_with_stages(vec![(
            "stack",
            true,
            HashMap::from([("payload".into(), json!(needle))]),
        )]);
        let mut specs = HashMap::new();
        specs.insert("stack".into(), spec("stack", 50));
        let report = validate_recipe_security(&recipe, &specs);
        let exec_violations = find_kind(&report, "executable");
        assert_eq!(
            exec_violations.len(),
            1,
            "expected 1 executable violation for '{}', got: {:?}",
            needle,
            exec_violations
        );
    }
}

#[test]
fn non_executable_string_does_not_trigger_executable_violation() {
    // A normal description string that happens to
    // contain the substring "evaluation" must not
    // trip the executable heuristic (case-sensitive
    // matching avoids that trap; "evaluation"
    // doesn't contain `eval(` anyway).
    let recipe = recipe_with_stages(vec![(
        "stack",
        true,
        HashMap::from([(
            "description".into(),
            json!("This is an evaluation of the input frames"),
        )]),
    )]);
    let mut specs = HashMap::new();
    specs.insert("stack".into(), spec("stack", 50));
    let report = validate_recipe_security(&recipe, &specs);
    let exec_violations = find_kind(&report, "executable");
    assert_eq!(exec_violations.len(), 0);
}

#[test]
fn resource_budget_over_max_produces_resource_violation() {
    // 4 stages at 100 units each = 400 > 300
    // ceiling. The recipe has 4 stages so we can
    // sum them.
    let recipe = recipe_with_stages(vec![
        ("a", true, HashMap::new()),
        ("b", true, HashMap::new()),
        ("c", true, HashMap::new()),
        ("d", true, HashMap::new()),
    ]);
    let mut specs = HashMap::new();
    specs.insert("a".into(), spec("a", 100));
    specs.insert("b".into(), spec("b", 100));
    specs.insert("c".into(), spec("c", 100));
    specs.insert("d".into(), spec("d", 100));
    let report = validate_recipe_security(&recipe, &specs);
    assert_eq!(report.total_resource_units, 400);
    let resource_violations = find_kind(&report, "resource");
    assert_eq!(resource_violations.len(), 1);
    assert!(
        resource_violations[0]
            .message
            .contains(&MAX_RESOURCE_UNITS.to_string()),
        "got message: {}",
        resource_violations[0].message
    );
}

#[test]
fn resource_budget_exactly_at_max_is_safe() {
    // 3 stages at 100 each = 300 = ceiling. The
    // check is `>` not `>=`, so this passes.
    let recipe = recipe_with_stages(vec![
        ("a", true, HashMap::new()),
        ("b", true, HashMap::new()),
        ("c", true, HashMap::new()),
    ]);
    let mut specs = HashMap::new();
    specs.insert("a".into(), spec("a", 100));
    specs.insert("b".into(), spec("b", 100));
    specs.insert("c".into(), spec("c", 100));
    let report = validate_recipe_security(&recipe, &specs);
    assert_eq!(report.total_resource_units, 300);
    let resource_violations = find_kind(&report, "resource");
    assert_eq!(resource_violations.len(), 0);
}

#[test]
fn disabled_stages_dont_contribute_to_resource_budget() {
    // 4 stages, 3 disabled. Only one enabled
    // contributes its cost.
    let recipe = recipe_with_stages(vec![
        ("a", true, HashMap::new()),
        ("b", false, HashMap::new()),
        ("c", false, HashMap::new()),
        ("d", false, HashMap::new()),
    ]);
    let mut specs = HashMap::new();
    specs.insert("a".into(), spec("a", 100));
    specs.insert("b".into(), spec("b", 100));
    specs.insert("c".into(), spec("c", 100));
    specs.insert("d".into(), spec("d", 100));
    let report = validate_recipe_security(&recipe, &specs);
    assert_eq!(
        report.total_resource_units, 100,
        "disabled stages must not contribute to the budget"
    );
}

#[test]
fn composition_surfaces_all_five_classes_in_one_pass() {
    // A single Recipe that fails every class at
    // once. The validation pipeline runs all
    // checks; the report should have at least
    // one violation per class.
    let mut a_params = HashMap::new();
    a_params.insert("radius".into(), json!(15.0)); // out of range
    a_params.insert("script".into(), json!("#!/bin/sh")); // executable

    let mut b_params = HashMap::new();
    b_params.insert("input_path".into(), json!("/etc/passwd")); // filesystem
    b_params.insert("payload".into(), json!("subprocess.run")); // executable

    let recipe = recipe_with_stages(vec![("a", true, a_params), ("b", true, b_params)]);
    let mut specs = HashMap::new();
    specs.insert(
        "a".into(),
        spec_with_range("a", "radius", ParamRange::bounded(0.0, 10.0), 200),
    );
    specs.insert(
        "b".into(),
        spec_with_required("b", &["ghost"], 200), // dependency: ghost missing
    );
    let report = validate_recipe_security(&recipe, &specs);

    // Every class surfaces at least one violation.
    assert!(
        !find_kind(&report, "range").is_empty(),
        "range violations: {:?}",
        report.violations
    );
    assert!(
        !find_kind(&report, "dependency").is_empty(),
        "dependency violations: {:?}",
        report.violations
    );
    assert!(
        !find_kind(&report, "filesystem").is_empty(),
        "filesystem violations: {:?}",
        report.violations
    );
    assert!(
        !find_kind(&report, "executable").is_empty(),
        "executable violations: {:?}",
        report.violations
    );
    assert!(
        !find_kind(&report, "resource").is_empty(),
        "resource violations: {:?}",
        report.violations
    );
    assert!(!report.is_safe());
}

#[test]
fn report_round_trips_through_serde() {
    let report = SecurityValidationReport {
        total_resource_units: 100,
        violations: vec![SecurityViolation {
            kind: "range".into(),
            stage_id: Some("stretch".into()),
            param_key: Some("radius".into()),
            message: "test".into(),
        }],
    };
    let json = serde_json::to_string(&report).expect("serialize");
    let back: SecurityValidationReport = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(report, back);
}

#[test]
fn param_range_contains_handles_half_infinite() {
    // min set, max unset: n above min is in range.
    let r = ParamRange {
        min: Some(0.0),
        max: None,
    };
    assert!(r.contains(0.0));
    assert!(r.contains(100.0));
    assert!(!r.contains(-1.0));

    // max set, min unset: n below max is in range.
    let r = ParamRange {
        min: None,
        max: Some(10.0),
    };
    assert!(r.contains(10.0));
    assert!(r.contains(-100.0));
    assert!(!r.contains(11.0));

    // both set: n within bounds is in range.
    let r = ParamRange::bounded(0.0, 10.0);
    assert!(r.contains(0.0));
    assert!(r.contains(5.0));
    assert!(r.contains(10.0));
    assert!(!r.contains(-0.01));
    assert!(!r.contains(10.01));
}

// ─── CR-08 §10.2 / §20: Recipe-level range tests ───
//
// The Guided tier's QualityTargets fields are
// enforced separately from the per-(stage, param)
// range check. The kind tag is `"recipe_level"` and
// the stage_id is the sentinel `RECIPE_LEVEL_STAGE_ID`
// so the UI panel can group the violations under a
// dedicated "Recipe" header.

#[test]
fn recipe_level_targets_ranges_define_three_fields() {
    let r = recipe_level_targets_ranges();
    // SNR in dB; sharpness + background smoothness
    // are normalised 0-1.
    assert!(r.contains_key("target_snr_db"));
    assert!(r.contains_key("target_sharpness"));
    assert!(r.contains_key("target_background_smoothness"));
    assert_eq!(r.len(), 3);
    // Spot-check the bounds.
    let snr = r.get("target_snr_db").unwrap();
    assert_eq!(snr.min, Some(20.0));
    assert_eq!(snr.max, Some(60.0));
    let sharp = r.get("target_sharpness").unwrap();
    assert_eq!(sharp.min, Some(0.0));
    assert_eq!(sharp.max, Some(1.0));
}

#[test]
fn safe_recipe_with_targets_passes_validation() {
    // All three QualityTargets fields set within
    // their declared ranges: no recipe_level
    // violations.
    let mut recipe = recipe_with_stages(vec![(
        "stretch",
        true,
        HashMap::from([("radius".into(), json!(3.0))]),
    )]);
    recipe.quality_targets = QualityTargets {
        target_snr_db: Some(40.0),
        target_sharpness: Some(0.7),
        target_background_smoothness: Some(0.85),
    };
    let mut specs = HashMap::new();
    specs.insert(
        "stretch".into(),
        spec_with_range("stretch", "radius", ParamRange::bounded(0.0, 10.0), 50),
    );
    let report = validate_recipe_security(&recipe, &specs);
    assert!(
        report.is_safe(),
        "Recipe with in-range targets must pass: {:?}",
        report.violations
    );
}

#[test]
fn recipe_level_snr_below_minimum_produces_violation() {
    let mut recipe = recipe_with_stages(vec![(
        "stretch",
        true,
        HashMap::from([("radius".into(), json!(3.0))]),
    )]);
    recipe.quality_targets = QualityTargets {
        target_snr_db: Some(15.0), // below 20 dB minimum
        target_sharpness: None,
        target_background_smoothness: None,
    };
    let mut specs = HashMap::new();
    specs.insert(
        "stretch".into(),
        spec_with_range("stretch", "radius", ParamRange::bounded(0.0, 10.0), 50),
    );
    let report = validate_recipe_security(&recipe, &specs);
    assert!(!report.is_safe());
    let rls = find_kind(&report, "recipe_level");
    assert_eq!(
        rls.len(),
        1,
        "expected one recipe_level violation: {:?}",
        report.violations
    );
    let v = rls[0];
    assert_eq!(v.stage_id.as_deref(), Some(RECIPE_LEVEL_STAGE_ID));
    assert_eq!(v.param_key.as_deref(), Some("target_snr_db"));
    assert!(v.message.contains("below the minimum 20"));
}

#[test]
fn recipe_level_sharpness_above_maximum_produces_violation() {
    let mut recipe = recipe_with_stages(vec![(
        "stretch",
        true,
        HashMap::from([("radius".into(), json!(3.0))]),
    )]);
    recipe.quality_targets = QualityTargets {
        target_snr_db: None,
        target_sharpness: Some(1.5), // above 1.0 maximum
        target_background_smoothness: None,
    };
    let mut specs = HashMap::new();
    specs.insert(
        "stretch".into(),
        spec_with_range("stretch", "radius", ParamRange::bounded(0.0, 10.0), 50),
    );
    let report = validate_recipe_security(&recipe, &specs);
    assert!(!report.is_safe());
    let rls = find_kind(&report, "recipe_level");
    assert_eq!(rls.len(), 1);
    let v = rls[0];
    assert_eq!(v.param_key.as_deref(), Some("target_sharpness"));
    assert!(v.message.contains("exceeds the maximum 1"));
}

#[test]
fn recipe_level_background_smoothness_out_of_range_produces_violation() {
    let mut recipe = recipe_with_stages(vec![(
        "stretch",
        true,
        HashMap::from([("radius".into(), json!(3.0))]),
    )]);
    recipe.quality_targets = QualityTargets {
        target_snr_db: None,
        target_sharpness: None,
        target_background_smoothness: Some(-0.1), // below 0.0 minimum
    };
    let mut specs = HashMap::new();
    specs.insert(
        "stretch".into(),
        spec_with_range("stretch", "radius", ParamRange::bounded(0.0, 10.0), 50),
    );
    let report = validate_recipe_security(&recipe, &specs);
    assert!(!report.is_safe());
    let rls = find_kind(&report, "recipe_level");
    assert_eq!(rls.len(), 1);
    let v = rls[0];
    assert_eq!(v.param_key.as_deref(), Some("target_background_smoothness"));
}

#[test]
fn recipe_level_none_fields_produce_no_violations() {
    // A Recipe with all three QualityTargets fields
    // unset (the default) must not produce a
    // recipe_level violation. This is the safe
    // path for legacy Recipes that predate §10.2.
    let recipe = recipe_with_stages(vec![(
        "stretch",
        true,
        HashMap::from([("radius".into(), json!(3.0))]),
    )]);
    let mut specs = HashMap::new();
    specs.insert(
        "stretch".into(),
        spec_with_range("stretch", "radius", ParamRange::bounded(0.0, 10.0), 50),
    );
    let report = validate_recipe_security(&recipe, &specs);
    assert!(
        find_kind(&report, "recipe_level").is_empty(),
        "None fields must pass: {:?}",
        report.violations
    );
}

#[test]
fn recipe_level_all_three_out_of_range_produces_three_violations() {
    // Composition: all three fields out of range
    // surfaces three separate recipe_level
    // violations in a single pass.
    let mut recipe = recipe_with_stages(vec![(
        "stretch",
        true,
        HashMap::from([("radius".into(), json!(3.0))]),
    )]);
    recipe.quality_targets = QualityTargets {
        target_snr_db: Some(100.0),               // exceeds 60 dB
        target_sharpness: Some(2.0),              // exceeds 1.0
        target_background_smoothness: Some(-0.5), // below 0.0
    };
    let mut specs = HashMap::new();
    specs.insert(
        "stretch".into(),
        spec_with_range("stretch", "radius", ParamRange::bounded(0.0, 10.0), 50),
    );
    let report = validate_recipe_security(&recipe, &specs);
    let rls = find_kind(&report, "recipe_level");
    assert_eq!(rls.len(), 3);
    // Every violation is anchored on the sentinel
    // stage_id so the UI panel can group them.
    for v in rls {
        assert_eq!(v.stage_id.as_deref(), Some(RECIPE_LEVEL_STAGE_ID));
    }
}
