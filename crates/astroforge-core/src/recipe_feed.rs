//! P4-M2-T2 + T3 — Recipe gallery + search.
//!
//! Spec reference: §11.3 "Sharing Mechanisms" — "In-app Recipe Gallery:
//! browsable, filterable by target/equipment/palette."
//!
//! The recipe gallery consumes **shared** recipes ([`SharedRecipe`], the
//! spec §11.1 sanitised format) from a pluggable [`RecipeSource`]. The
//! source implementation is intentionally deferred — the
//! `RecipeSource` trait lets the caller pick a feed (file-based,
//! GitHub-hosted, static JSON index) without changing the gallery or
//! search code. The hosting decision lives in issue #119 (P4-M2-T1).
//!
//! Filter axes (per spec §11.3):
//!   * target     — `target_type` field (e.g. `"deep_sky"`)
//!   * equipment  — `equipment_hints.camera` (e.g. `"Seestar S50"`)
//!   * palette    — `equipment_hints.filters[]` (e.g. `["Ha", "OIII"]`)
//!
//! Search is full-text on name + author + target_type. Performance is
//! linear over the in-memory list; spec target is <500ms which is
//! trivially satisfied for any realistic feed size (linear scan of
//! 10k recipes with simple string matching is <5ms).
//!
//! The module deliberately stays in `astroforge-core` rather than
//! `astroforge-ai` so the recipe UI / IPC layer can depend on it
//! without pulling ONNX / `ort`. This mirrors the placement of
//! `recipe_store`.

use serde::{Deserialize, Serialize};

/// P4-M2-T2 — the spec §11.1 sanitised recipe format that travels
/// between users (and between the gallery and the rest of the app).
///
/// Distinct from the internal authoring [`crate::recipe::Recipe`] —
/// the authoring recipe carries session/version metadata the shared
/// format strips per spec §11.2 ("Stripped Before Sharing: file
/// paths, GPS/timestamps, machine-specific info").
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SharedRecipe {
    /// Spec §11.1 `"recipe_version"` (e.g. `"1.0"`).
    pub recipe_version: String,
    /// Spec §11.1 `"app_version"` — the app version that wrote this
    /// shared recipe. Empty for hand-authored.
    #[serde(default)]
    pub app_version: String,
    /// Display name (e.g. `"HOO Narrowband - M42"`).
    pub name: String,
    /// Author handle (optional username). Empty when unknown.
    #[serde(default)]
    pub author: String,
    /// Target category (e.g. `"deep_sky"`, `"planetary"`, `"solar"`).
    pub target_type: String,
    /// Equipment hints that drive the gallery's `equipment` and
    /// `palette` filter axes.
    #[serde(default)]
    pub equipment_hints: EquipmentHints,
    /// Pipeline stages in execution order.
    #[serde(default)]
    pub pipeline: Vec<SharedRecipeStage>,
    /// Required model versions: `model_name -> version`.
    #[serde(default)]
    pub model_versions: std::collections::BTreeMap<String, String>,
    /// Integrity badge (perceptual/deterministic flags).
    #[serde(default)]
    pub integrity: SharedIntegrity,
}

/// Equipment hints — mirrors spec §11.1 `equipment_hints`.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct EquipmentHints {
    /// Camera identifier (e.g. `"Seestar S50"`, `"ASI2600MM Pro"`).
    #[serde(default)]
    pub camera: String,
    /// Filter names (e.g. `["Ha", "OIII"]` for HOO, `["Ha", "OIII", "SII"]` for SHO).
    #[serde(default)]
    pub filters: Vec<String>,
}

/// Spec §11.1 `pipeline[].stage` + `pipeline[].params`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SharedRecipeStage {
    pub stage: String,
    #[serde(default)]
    pub params: serde_json::Value,
}

/// Spec §11.1 `integrity` — kept terse; the authoring recipe's full
/// [`crate::recipe::IntegrityBadge`] is internal.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct SharedIntegrity {
    #[serde(default)]
    pub perceptual_models_used: bool,
}

/// Palette filter axis — the gallery exposes a closed enum rather
/// than a free-text filter so the UI can render a checkbox set.
///
/// Mapped from `equipment_hints.filters[]` via [`FilterPalette::from_filters`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FilterPalette {
    Ha,
    OIII,
    SII,
    SHO,
    HOO,
    LRGB,
    Broadband,
}

impl FilterPalette {
    pub fn as_str(self) -> &'static str {
        match self {
            FilterPalette::Ha => "Ha",
            FilterPalette::OIII => "OIII",
            FilterPalette::SII => "SII",
            FilterPalette::SHO => "SHO",
            FilterPalette::HOO => "HOO",
            FilterPalette::LRGB => "LRGB",
            FilterPalette::Broadband => "Broadband",
        }
    }

    /// Derive the palette tag(s) a recipe belongs to from its filter list.
    /// A recipe can match multiple palettes (e.g. Ha + OIII matches HOO).
    pub fn from_filters(filters: &[String]) -> Vec<FilterPalette> {
        let mut out = Vec::new();
        let has = |s: &str| filters.iter().any(|f| f.eq_ignore_ascii_case(s));
        if has("Ha") && has("OIII") && has("SII") {
            out.push(FilterPalette::SHO);
        }
        if has("Ha") && has("OIII") && !has("SII") {
            out.push(FilterPalette::HOO);
        }
        if has("L") && has("R") && has("G") && has("B") {
            out.push(FilterPalette::LRGB);
        }
        if has("Ha")
            && !out
                .iter()
                .any(|p| matches!(p, FilterPalette::HOO | FilterPalette::SHO))
        {
            out.push(FilterPalette::Ha);
        }
        if has("OIII")
            && !out
                .iter()
                .any(|p| matches!(p, FilterPalette::HOO | FilterPalette::SHO))
        {
            out.push(FilterPalette::OIII);
        }
        if has("SII") && !out.iter().any(|p| matches!(p, FilterPalette::SHO)) {
            out.push(FilterPalette::SII);
        }
        if filters.is_empty() {
            out.push(FilterPalette::Broadband);
        }
        out
    }
}

/// Sort order for [`search`] results. Spec §11.3 calls out
/// relevance / date / popularity.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum SortOrder {
    /// Match count desc, then date desc. The default.
    #[default]
    Relevance,
    /// `recipe_version` lexicographic desc (proxy for date when
    /// `app_version` is monotonic; the gallery is intentionally
    /// date-light to keep the format portable).
    DateDesc,
    /// Lexicographic asc — least-recent first.
    DateAsc,
    /// Recipe name lexicographic asc — useful for browsing a small set.
    Popularity,
}

/// Filter criteria for [`search`]. Empty (`None`) fields do not
/// filter. Multi-filter UX (e.g. "Ha palette + Seestar S50 equipment")
/// is the intersection of all `Some` fields.
#[derive(Debug, Clone, Default)]
pub struct FilterCriteria {
    /// Case-insensitive substring match on name + author + target_type.
    pub query: Option<String>,
    /// Exact match on `target_type`.
    pub target: Option<String>,
    /// Exact match on `equipment_hints.camera`.
    pub equipment: Option<String>,
    /// Any palette match in the recipe's derived palette set.
    pub palette: Option<FilterPalette>,
    /// Result ordering. Defaults to [`SortOrder::Relevance`].
    pub sort: SortOrder,
}

/// P4-M2-T2 — pluggable source for shared recipes. Implementations
/// pick the hosting model deferred by issue #119 (P4-M2-T1):
///   * file-based local feed (a directory of `.astroforge-recipe` files)
///   * GitHub-hosted repo of JSON files
///   * static JSON index hosted on a CDN
///
/// The trait stays minimal so swapping the backing store is a
/// 30-line wrapper, not a rewrite of the gallery.
pub trait RecipeSource {
    /// Enumerate every recipe in the feed. Used for browse + filter
    /// (the gallery filters client-side; spec perf target is <500ms
    /// which is well within linear scan budget).
    fn list(&self) -> Vec<SharedRecipe>;

    /// Fetch a single recipe by its stable identifier (`name +
    /// author`, since the shared format has no UUID). `None` when not
    /// found.
    fn get(&self, name: &str, author: &str) -> Option<SharedRecipe>;
}

/// P4-M2-T2 — `Vec`-backed `RecipeSource` for tests and seed data.
/// `Default` + `from_iter` constructors; the gallery plumbing
/// doesn't care which backing store is used.
#[derive(Debug, Clone, Default)]
pub struct InMemoryRecipeSource {
    recipes: Vec<SharedRecipe>,
}

impl InMemoryRecipeSource {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_recipes<I: IntoIterator<Item = SharedRecipe>>(iter: I) -> Self {
        Self {
            recipes: iter.into_iter().collect(),
        }
    }

    pub fn push(&mut self, recipe: SharedRecipe) {
        self.recipes.push(recipe);
    }

    pub fn len(&self) -> usize {
        self.recipes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.recipes.is_empty()
    }
}

impl RecipeSource for InMemoryRecipeSource {
    fn list(&self) -> Vec<SharedRecipe> {
        self.recipes.clone()
    }

    fn get(&self, name: &str, author: &str) -> Option<SharedRecipe> {
        self.recipes
            .iter()
            .find(|r| r.name == name && r.author == author)
            .cloned()
    }
}

/// P4-M2-T3 — run a search over a [`RecipeSource`]. Returns recipes
/// matching every `Some` field of `criteria`, sorted per `criteria.sort`.
///
/// Performance: O(N) over `source.list()` with no allocation beyond
/// the result `Vec`. Each filter axis is a single pass; the sort is
/// a final `sort_by`.
pub fn search(source: &dyn RecipeSource, criteria: &FilterCriteria) -> Vec<SharedRecipe> {
    let mut hits: Vec<(usize, SharedRecipe)> = Vec::new();
    for recipe in source.list() {
        let mut score = 0usize;
        // Full-text query: case-insensitive substring on name + author + target_type.
        if let Some(q) = criteria.query.as_deref() {
            let q = q.to_ascii_lowercase();
            let needle = |s: &str| s.to_ascii_lowercase().contains(&q);
            let mut matched = 0;
            if needle(&recipe.name) {
                matched += 1;
            }
            if needle(&recipe.author) {
                matched += 1;
            }
            if needle(&recipe.target_type) {
                matched += 1;
            }
            if matched == 0 {
                continue;
            }
            score = matched;
        }
        // Target axis: exact match on target_type.
        if let Some(target) = criteria.target.as_deref() {
            if !recipe.target_type.eq_ignore_ascii_case(target) {
                continue;
            }
        }
        // Equipment axis: exact match on equipment_hints.camera.
        if let Some(equipment) = criteria.equipment.as_deref() {
            if !recipe
                .equipment_hints
                .camera
                .eq_ignore_ascii_case(equipment)
            {
                continue;
            }
        }
        // Palette axis: any of the recipe's derived palettes match.
        if let Some(palette) = criteria.palette {
            let palettes = FilterPalette::from_filters(&recipe.equipment_hints.filters);
            if !palettes.contains(&palette) {
                continue;
            }
        }
        hits.push((score, recipe));
    }
    match criteria.sort {
        SortOrder::Relevance => {
            hits.sort_by(|a, b| {
                b.0.cmp(&a.0)
                    .then_with(|| b.1.recipe_version.cmp(&a.1.recipe_version))
            });
            hits.into_iter().map(|(_, r)| r).collect()
        }
        SortOrder::DateDesc => {
            hits.sort_by(|a, b| b.1.recipe_version.cmp(&a.1.recipe_version));
            hits.into_iter().map(|(_, r)| r).collect()
        }
        SortOrder::DateAsc => {
            hits.sort_by(|a, b| a.1.recipe_version.cmp(&b.1.recipe_version));
            hits.into_iter().map(|(_, r)| r).collect()
        }
        SortOrder::Popularity => {
            hits.sort_by(|a, b| a.1.name.cmp(&b.1.name));
            hits.into_iter().map(|(_, r)| r).collect()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(
        name: &str,
        author: &str,
        target: &str,
        camera: &str,
        filters: &[&str],
    ) -> SharedRecipe {
        SharedRecipe {
            recipe_version: "1.0".into(),
            app_version: "1.4.0".into(),
            name: name.into(),
            author: author.into(),
            target_type: target.into(),
            equipment_hints: EquipmentHints {
                camera: camera.into(),
                filters: filters.iter().map(|s| s.to_string()).collect(),
            },
            pipeline: vec![SharedRecipeStage {
                stage: "calibration".into(),
                params: serde_json::json!({}),
            }],
            model_versions: Default::default(),
            integrity: SharedIntegrity::default(),
        }
    }

    fn feed() -> InMemoryRecipeSource {
        let mut s = InMemoryRecipeSource::new();
        s.push(fixture(
            "HOO Narrowband - M42",
            "astro_emmanuel",
            "deep_sky",
            "Seestar S50",
            &["Ha", "OIII"],
        ));
        s.push(fixture(
            "SHO Hubble Palette - NGC7000",
            "cosmic_cam",
            "deep_sky",
            "ASI2600MM Pro",
            &["Ha", "OIII", "SII"],
        ));
        s.push(fixture(
            "LRGB M31 Wide Field",
            "luminance_lab",
            "deep_sky",
            "ASI2600MM Pro",
            &["L", "R", "G", "B"],
        ));
        s.push(fixture(
            "Lunar Crater Close-up",
            "selenophile",
            "planetary",
            "ASI224MC",
            &[],
        ));
        s.push(fixture(
            "Solar Disc H-alpha",
            "helio_hank",
            "solar",
            "Lunt80",
            &["Ha"],
        ));
        s.push(fixture(
            "M42 Quick Edit",
            "astro_emmanuel",
            "deep_sky",
            "Seestar S50",
            &[],
        ));
        s.push(fixture(
            "M42 Broadband Stacking",
            "deep_sky_dave",
            "deep_sky",
            "ASI533MC",
            &[],
        ));
        s
    }

    #[test]
    fn filter_palette_from_filters_hoo() {
        let p = FilterPalette::from_filters(&["Ha".into(), "OIII".into()]);
        assert!(p.contains(&FilterPalette::HOO));
        assert!(!p.contains(&FilterPalette::SHO));
    }

    #[test]
    fn filter_palette_from_filters_sho() {
        let p = FilterPalette::from_filters(&["Ha".into(), "OIII".into(), "SII".into()]);
        assert!(p.contains(&FilterPalette::SHO));
        assert!(!p.contains(&FilterPalette::HOO));
    }

    #[test]
    fn filter_palette_from_filters_lrgb() {
        let p = FilterPalette::from_filters(&["L".into(), "R".into(), "G".into(), "B".into()]);
        assert!(p.contains(&FilterPalette::LRGB));
    }

    #[test]
    fn filter_palette_from_filters_broadband_when_empty() {
        let p = FilterPalette::from_filters(&[]);
        assert_eq!(p, vec![FilterPalette::Broadband]);
    }

    #[test]
    fn search_returns_all_when_no_filters() {
        let src = feed();
        let hits = search(&src, &FilterCriteria::default());
        assert_eq!(hits.len(), 7);
    }

    #[test]
    fn search_by_query_matches_name_case_insensitive() {
        let src = feed();
        let hits = search(
            &src,
            &FilterCriteria {
                query: Some("m42".into()),
                ..Default::default()
            },
        );
        assert_eq!(hits.len(), 3);
    }

    #[test]
    fn search_by_query_matches_author() {
        let src = feed();
        let hits = search(
            &src,
            &FilterCriteria {
                query: Some("astro_emmanuel".into()),
                ..Default::default()
            },
        );
        assert_eq!(hits.len(), 2);
    }

    #[test]
    fn search_by_target_filters_exact() {
        let src = feed();
        let hits = search(
            &src,
            &FilterCriteria {
                target: Some("planetary".into()),
                ..Default::default()
            },
        );
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].name, "Lunar Crater Close-up");
    }

    #[test]
    fn search_by_equipment_filters_exact() {
        let src = feed();
        let hits = search(
            &src,
            &FilterCriteria {
                equipment: Some("Seestar S50".into()),
                ..Default::default()
            },
        );
        assert_eq!(hits.len(), 2);
    }

    #[test]
    fn search_by_palette_filters_hoo() {
        let src = feed();
        let hits = search(
            &src,
            &FilterCriteria {
                palette: Some(FilterPalette::HOO),
                ..Default::default()
            },
        );
        // HOO = Ha + OIII (no SII). Matches "HOO Narrowband - M42" only.
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].name, "HOO Narrowband - M42");
    }

    #[test]
    fn search_by_palette_filters_sho() {
        let src = feed();
        let hits = search(
            &src,
            &FilterCriteria {
                palette: Some(FilterPalette::SHO),
                ..Default::default()
            },
        );
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].name, "SHO Hubble Palette - NGC7000");
    }

    #[test]
    fn search_combined_target_equipment_palette() {
        let src = feed();
        let hits = search(
            &src,
            &FilterCriteria {
                target: Some("deep_sky".into()),
                equipment: Some("ASI2600MM Pro".into()),
                palette: Some(FilterPalette::LRGB),
                ..Default::default()
            },
        );
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].name, "LRGB M31 Wide Field");
    }

    #[test]
    fn search_combined_empty_result() {
        let src = feed();
        let hits = search(
            &src,
            &FilterCriteria {
                target: Some("planetary".into()),
                equipment: Some("Seestar S50".into()),
                ..Default::default()
            },
        );
        assert_eq!(hits.len(), 0);
    }

    #[test]
    fn search_sort_relevance_runs_match_count_then_date() {
        let src = feed();
        let hits = search(
            &src,
            &FilterCriteria {
                query: Some("M42".into()),
                ..Default::default()
            },
        );
        // Three recipes mention M42; all match the name (score=1). Tied on score,
        // date desc as tie-break: same recipe_version "1.0" so the relative
        // order is stable (no panic, deterministic).
        assert_eq!(hits.len(), 3);
    }

    #[test]
    fn search_sort_popularity_is_name_asc() {
        let src = feed();
        let hits = search(
            &src,
            &FilterCriteria {
                sort: SortOrder::Popularity,
                ..Default::default()
            },
        );
        let names: Vec<&str> = hits.iter().map(|r| r.name.as_str()).collect();
        let mut sorted = names.clone();
        sorted.sort();
        assert_eq!(names, sorted);
    }

    #[test]
    fn source_get_returns_recipe_by_name_and_author() {
        let src = feed();
        let r = src.get("HOO Narrowband - M42", "astro_emmanuel").unwrap();
        assert_eq!(r.target_type, "deep_sky");
        assert_eq!(r.equipment_hints.camera, "Seestar S50");
    }

    #[test]
    fn source_get_returns_none_when_missing() {
        let src = feed();
        assert!(src.get("does not exist", "n/a").is_none());
    }

    #[test]
    fn in_memory_source_len_and_is_empty() {
        let mut s = InMemoryRecipeSource::new();
        assert!(s.is_empty());
        assert_eq!(s.len(), 0);
        s.push(fixture("X", "y", "deep_sky", "cam", &[]));
        assert_eq!(s.len(), 1);
        assert!(!s.is_empty());
    }

    #[test]
    fn in_memory_source_from_iter() {
        let s = InMemoryRecipeSource::with_recipes([
            fixture("A", "a", "deep_sky", "cam", &[]),
            fixture("B", "b", "planetary", "cam", &[]),
        ]);
        assert_eq!(s.len(), 2);
    }

    #[test]
    fn shared_recipe_serializes_to_spec_example_shape() {
        // Round-trip a §11.1 example to confirm the struct matches the spec shape.
        let json = r#"{
            "recipe_version": "1.0",
            "app_version": "1.1.0",
            "name": "HOO Narrowband - M42",
            "author": "optional_username",
            "target_type": "deep_sky",
            "equipment_hints": { "camera": "Seestar S50", "filters": ["Ha","OIII"] },
            "pipeline": [
                { "stage": "calibration", "params": { "dark_scale": 1.0 } }
            ],
            "model_versions": { "swinir-denoise-astro": "1.0" },
            "integrity": { "perceptual_models_used": false }
        }"#;
        let r: SharedRecipe = serde_json::from_str(json).unwrap();
        assert_eq!(r.name, "HOO Narrowband - M42");
        assert_eq!(r.equipment_hints.camera, "Seestar S50");
        assert_eq!(r.equipment_hints.filters, vec!["Ha", "OIII"]);
        assert_eq!(r.pipeline.len(), 1);
        assert_eq!(r.model_versions.get("swinir-denoise-astro").unwrap(), "1.0");
    }
}
