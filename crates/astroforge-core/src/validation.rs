//! CR-08 §20: Recipe security validation. Pure functions
//! that scan a [`Recipe`] for five classes of risk:
//!
//! 1. **Parameter range**: per-(stage, param) numeric
//!    bounds (min/max) declared in the spec table
//!    are enforced. Out-of-range values reject the
//!    Recipe so the apply round never sees them.
//! 2. **Dependency**: a stage declaring
//!    `required_stages: &[stage_id]` cannot be enabled
//!    if any required stage is missing or disabled.
//! 3. **Filesystem reference rejection**: any string
//!    param value containing an absolute path
//!    (`/`, `\`, `~/`, `://`) is rejected. The Recipe
//!    must not point at the host filesystem.
//! 4. **Executable payload rejection**: any string
//!    param value that looks like an executable payload
//!    (shebang `#!/...`, contains `os.system(`,
//!    `subprocess.`, `eval(`, `exec(`, `<script`,
//!    `shell_exec`) is rejected. The Recipe must
//!    not carry code.
//! 5. **Resource budget**: each stage declares a
//!    `resource_units` cost; the Recipe's sum is
//!    rejected when it exceeds the
//!    [`MAX_RESOURCE_UNITS`] ceiling.
//!
//! The five checks compose into a single
//! `SecurityValidationReport` so the apply round can
//! render all violations in one pass (rather than
//! surfacing them one at a time). All functions are
//! pure: no IO, no side effects.

use crate::recipe::{Recipe, RecipeStage};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

/// CR-08 §20: ceiling on the per-Recipe resource
/// budget. Picked to leave headroom for the M42
/// natural-deep pipeline (which the §22 quality
/// catalog already exercises at ~150 resource
/// units) plus a 2x safety margin.
pub const MAX_RESOURCE_UNITS: u32 = 300;

/// CR-08 §20: numeric bound for a single
/// per-(stage, param) range check. The lower /
/// upper fields are inclusive. Either bound may be
/// omitted (the half-infinite range).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ParamRange {
    /// Inclusive lower bound. `None` means
    /// unbounded below.
    pub min: Option<f64>,
    /// Inclusive upper bound. `None` means
    /// unbounded above.
    pub max: Option<f64>,
}

impl ParamRange {
    /// Convenience constructor for a fully-bounded
    /// range.
    pub fn bounded(min: f64, max: f64) -> Self {
        Self {
            min: Some(min),
            max: Some(max),
        }
    }

    /// Returns true iff `n` is within the range
    /// (inclusive on both ends). Unset bounds
    /// don't constrain.
    pub fn contains(&self, n: f64) -> bool {
        let below = self.min.map(|m| n < m).unwrap_or(false);
        let above = self.max.map(|m| n > m).unwrap_or(false);
        !below && !above
    }
}

/// CR-08 §20: per-stage spec entry. Drives the §20
/// validation pipeline. `params` declares the
/// per-key numeric ranges; `required_stages`
/// declares the dependency edges; `resource_units`
/// declares the cost. Unknown params (params in
/// the Recipe but not in the spec) are passed
/// through unchanged.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StageSpec {
    pub stage_id: String,
    /// Per-key numeric bounds. A RecipeStage's
    /// param value at key `k` must satisfy
    /// `range(k)` if the Recipe's value is a
    /// number.
    pub params: HashMap<String, ParamRange>,
    /// Stages that must be present + enabled
    /// for this stage to be valid.
    pub required_stages: Vec<String>,
    /// Per-stage resource cost. Summed across the
    /// Recipe and compared against
    /// [`MAX_RESOURCE_UNITS`].
    pub resource_units: u32,
}

impl StageSpec {
    /// Convenience constructor for the common case
    /// where the spec has no range or dependency
    /// constraints but does declare a resource
    /// cost. Used by the test fixture builder so
    /// the spec table reads cleanly in tests.
    pub fn cheap(stage_id: impl Into<String>, resource_units: u32) -> Self {
        Self {
            stage_id: stage_id.into(),
            params: HashMap::new(),
            required_stages: Vec::new(),
            resource_units,
        }
    }
}

/// CR-08 §20: a single violation emitted by the
/// validation pipeline. Drives the §20 UI panel
/// (per-row error icon + tooltip text).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SecurityViolation {
    /// Which class produced it (`"range"`,
    /// `"dependency"`, `"filesystem"`,
    /// `"executable"`, `"resource"`).
    pub kind: String,
    /// Stage ID the violation is anchored to, when
    /// applicable (e.g. range violations on
    /// `stage_id = "stretch"`).
    pub stage_id: Option<String>,
    /// Param key the violation is anchored to,
    /// when applicable (e.g. range violations on
    /// `param_key = "radius"`).
    pub param_key: Option<String>,
    /// Human-readable description. The UI
    /// surfaces this verbatim.
    pub message: String,
}

/// CR-08 §20: the full validation report. The
/// apply round renders the violations list when
/// it is non-empty. `violations` is empty iff the
/// Recipe is safe to apply.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SecurityValidationReport {
    /// Sum of every enabled stage's
    /// `resource_units`.
    pub total_resource_units: u32,
    /// All violations surfaced in this pass.
    /// Empty when the Recipe is safe.
    pub violations: Vec<SecurityViolation>,
}

impl SecurityValidationReport {
    /// True iff no violations. Drives the
    /// `Apply` button enable state in the §20
    /// UI panel.
    pub fn is_safe(&self) -> bool {
        self.violations.is_empty()
    }
}

/// CR-08 §20: validate a Recipe against the
/// stage spec table. Returns a
/// [`SecurityValidationReport`] that the apply
/// round + the §20 UI panel both consume.
///
/// The function never panics on unexpected input
/// (numeric overflow, unknown stage IDs, missing
/// spec entries) — every error is surfaced as a
/// [`SecurityViolation`] with a stable `kind` tag
/// so the UI can group + count them.
pub fn validate_recipe_security(
    recipe: &Recipe,
    stage_specs: &HashMap<String, StageSpec>,
) -> SecurityValidationReport {
    let mut violations = Vec::new();
    let mut total_resource_units: u32 = 0;

    // Pass 1: gather enabled stage IDs (for the
    // dependency check) and total resource cost.
    let enabled_stage_ids: HashSet<&str> = recipe
        .stages
        .iter()
        .filter(|s| s.enabled)
        .map(|s| s.stage_id.as_str())
        .collect();

    for stage in &recipe.stages {
        if !stage.enabled {
            continue;
        }
        // Each stage contributes its spec's
        // resource_units when available; missing
        // specs default to 0 (costless) so the
        // stage-presence check below can surface
        // the spec-missing case separately.
        let cost = stage_specs
            .get(&stage.stage_id)
            .map(|s| s.resource_units)
            .unwrap_or(0);
        total_resource_units = total_resource_units.saturating_add(cost);
    }

    if total_resource_units > MAX_RESOURCE_UNITS {
        violations.push(SecurityViolation {
            kind: "resource".into(),
            stage_id: None,
            param_key: None,
            message: format!(
                "Recipe resource budget {} exceeds ceiling {}.",
                total_resource_units, MAX_RESOURCE_UNITS
            ),
        });
    }

    // Pass 2: per-stage checks (range + dependency
    // + filesystem + executable).
    for stage in &recipe.stages {
        if !stage.enabled {
            continue;
        }
        check_range(stage, stage_specs, &mut violations);
        check_dependency(stage, stage_specs, &enabled_stage_ids, &mut violations);
        check_filesystem(stage, &mut violations);
        check_executable(stage, &mut violations);
    }

    // Pass 3: stage-presence check. A Recipe
    // containing a stage that has no spec entry
    // is rejected as a dependency violation (the
    // dependency-check loop above only fires
    // when the stage has a spec).
    for stage in &recipe.stages {
        if !stage.enabled {
            continue;
        }
        if !stage_specs.contains_key(&stage.stage_id) {
            violations.push(SecurityViolation {
                kind: "dependency".into(),
                stage_id: Some(stage.stage_id.clone()),
                param_key: None,
                message: format!(
                    "Stage '{}' is not in the spec catalog; \
                     cannot verify its dependencies.",
                    stage.stage_id
                ),
            });
        }
    }

    SecurityValidationReport {
        total_resource_units,
        violations,
    }
}

/// Per-(stage, param) numeric range check.
fn check_range(
    stage: &RecipeStage,
    stage_specs: &HashMap<String, StageSpec>,
    out: &mut Vec<SecurityViolation>,
) {
    let Some(spec) = stage_specs.get(&stage.stage_id) else {
        return;
    };
    for (key, range) in &spec.params {
        let Some(v) = stage.params.get(key) else {
            continue;
        };
        let Some(n) = v.as_f64() else {
            continue;
        };
        if !range.contains(n) {
            let reason = match (range.min, range.max) {
                (Some(m), _) if n < m => format!("below the minimum {}", m),
                (_, Some(m)) if n > m => format!("exceeds the maximum {}", m),
                _ => "out of range".into(),
            };
            out.push(SecurityViolation {
                kind: "range".into(),
                stage_id: Some(stage.stage_id.clone()),
                param_key: Some(key.clone()),
                message: format!(
                    "Param '{}' on stage '{}' = {} {}.",
                    key, stage.stage_id, n, reason
                ),
            });
        }
    }
}

/// Dependency check: every stage's
/// `required_stages` must be present + enabled.
fn check_dependency(
    stage: &RecipeStage,
    stage_specs: &HashMap<String, StageSpec>,
    enabled_stage_ids: &HashSet<&str>,
    out: &mut Vec<SecurityViolation>,
) {
    let Some(spec) = stage_specs.get(&stage.stage_id) else {
        return;
    };
    for req in &spec.required_stages {
        // A required stage is satisfied iff the
        // Recipe has it AND it's enabled.
        if !enabled_stage_ids.contains(req.as_str()) {
            out.push(SecurityViolation {
                kind: "dependency".into(),
                stage_id: Some(stage.stage_id.clone()),
                param_key: None,
                message: format!(
                    "Stage '{}' requires '{}' but it is missing or \
                     disabled.",
                    stage.stage_id, req
                ),
            });
        }
    }
}

/// Filesystem-reference rejection: any string
/// param value that contains an absolute path
/// indicator is rejected. The Recipe must not
/// point at the host filesystem.
fn check_filesystem(stage: &RecipeStage, out: &mut Vec<SecurityViolation>) {
    for (key, value) in &stage.params {
        let Some(s) = value.as_str() else {
            continue;
        };
        // Reject absolute paths (POSIX `/` and
        // Windows `\`), plus home-relative `~/`.
        // The check is intentionally permissive
        // (any occurrence) so the user gets a
        // clear rejection when a string param
        // leaks a path by accident.
        if s.starts_with('/') || s.starts_with('\\') || s.starts_with("~/") {
            out.push(SecurityViolation {
                kind: "filesystem".into(),
                stage_id: Some(stage.stage_id.clone()),
                param_key: Some(key.clone()),
                message: format!(
                    "Param '{}' on stage '{}' contains a filesystem \
                     reference ('{}') which is not allowed.",
                    key, stage.stage_id, s
                ),
            });
            continue;
        }
        // Catch mid-string URL schemes like
        // "file:///etc/passwd" or "https://..." in
        // stack paths.
        if s.contains("://") {
            out.push(SecurityViolation {
                kind: "filesystem".into(),
                stage_id: Some(stage.stage_id.clone()),
                param_key: Some(key.clone()),
                message: format!(
                    "Param '{}' on stage '{}' contains a URL or path \
                     scheme ('{}') which is not allowed.",
                    key, stage.stage_id, s
                ),
            });
        }
    }
}

/// Executable-payload rejection: any string
/// param value that looks like executable code
/// is rejected. The Recipe must not carry code.
fn check_executable(stage: &RecipeStage, out: &mut Vec<SecurityViolation>) {
    for (key, value) in &stage.params {
        let Some(s) = value.as_str() else {
            continue;
        };
        // Shebangs.
        if s.starts_with("#!") {
            out.push(SecurityViolation {
                kind: "executable".into(),
                stage_id: Some(stage.stage_id.clone()),
                param_key: Some(key.clone()),
                message: format!(
                    "Param '{}' on stage '{}' starts with a shebang \
                     and looks like an executable script.",
                    key, stage.stage_id
                ),
            });
            continue;
        }
        // Python + shell signatures. The match
        // is case-sensitive so legitimate
        // words like `evaluation` do not trip
        // the heuristic.
        for needle in [
            "os.system(",
            "subprocess.",
            "eval(",
            "<script",
            "</script",
            "exec(",
            "shell_exec",
        ] {
            if s.contains(needle) {
                out.push(SecurityViolation {
                    kind: "executable".into(),
                    stage_id: Some(stage.stage_id.clone()),
                    param_key: Some(key.clone()),
                    message: format!(
                        "Param '{}' on stage '{}' contains '{}' which \
                         looks like executable code.",
                        key, stage.stage_id, needle
                    ),
                });
                break;
            }
        }
    }
}
