use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Per-stage parameter map produced by [`apply_recipe`].
pub type StageParams = Vec<(String, HashMap<String, serde_json::Value>)>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Recipe {
    pub schema_version: String,
    pub name: String,
    pub description: String,
    pub target_type: String,
    pub stages: Vec<RecipeStage>,
    pub required_models: Vec<String>,
    pub integrity: IntegrityBadge,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecipeStage {
    pub stage_id: String,
    pub enabled: bool,
    pub params: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntegrityBadge {
    pub perceptual_models_used: bool,
    pub deterministic_models_used: bool,
    pub seed_recorded: bool,
    pub models: Vec<ModelUsage>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelUsage {
    pub model_name: String,
    pub model_type: ModelType,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ModelType {
    Deterministic,
    Perceptual,
}

impl Recipe {
    pub fn new(name: &str, target_type: &str) -> Self {
        Self {
            schema_version: "1.0".into(),
            name: name.into(),
            description: String::new(),
            target_type: target_type.into(),
            stages: Vec::new(),
            required_models: Vec::new(),
            integrity: IntegrityBadge {
                perceptual_models_used: false,
                deterministic_models_used: false,
                seed_recorded: false,
                models: Vec::new(),
            },
        }
    }

    pub fn add_stage(&mut self, stage_id: &str, params: HashMap<String, serde_json::Value>) {
        self.stages.push(RecipeStage {
            stage_id: stage_id.into(),
            enabled: true,
            params,
        });
    }

    pub fn add_model(&mut self, name: &str, model_type: ModelType) {
        self.required_models.push(name.into());
        self.integrity.models.push(ModelUsage {
            model_name: name.into(),
            model_type: model_type.clone(),
        });
        match model_type {
            ModelType::Deterministic => self.integrity.deterministic_models_used = true,
            ModelType::Perceptual => self.integrity.perceptual_models_used = true,
        }
    }

    pub fn set_seed_recorded(&mut self, recorded: bool) {
        self.integrity.seed_recorded = recorded;
    }

    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }
}

pub fn sanitize_recipe(recipe: &mut Recipe) {
    for stage in &mut recipe.stages {
        let keys_to_remove: Vec<String> = stage
            .params
            .iter()
            .filter(|(k, _)| {
                let key = k.to_lowercase();
                key.contains("path")
                    || key.contains("dir")
                    || key.contains("gps")
                    || key.contains("lat")
                    || key.contains("lon")
                    || key.contains("machine")
                    || key.contains("hostname")
                    || key.contains("user")
            })
            .map(|(k, _)| k.clone())
            .collect();

        for key in keys_to_remove {
            stage.params.remove(&key);
        }
    }
}

pub fn validate_compatibility(recipe: &Recipe, available_models: &[String]) -> ValidationResult {
    let missing: Vec<String> = recipe
        .required_models
        .iter()
        .filter(|m| !available_models.contains(m))
        .cloned()
        .collect();

    if !missing.is_empty() {
        return ValidationResult::MissingModels(missing);
    }

    if recipe.schema_version != "1.0" {
        return ValidationResult::IncompatibleVersion(recipe.schema_version.clone());
    }

    ValidationResult::Compatible
}

#[derive(Debug, Clone, PartialEq)]
pub enum ValidationResult {
    Compatible,
    MissingModels(Vec<String>),
    IncompatibleVersion(String),
}

pub fn apply_recipe(
    recipe: &Recipe,
    available_models: &[String],
) -> Result<StageParams, ApplyError> {
    match validate_compatibility(recipe, available_models) {
        ValidationResult::Compatible => {}
        ValidationResult::MissingModels(missing) => {
            return Err(ApplyError::MissingModels(missing));
        }
        ValidationResult::IncompatibleVersion(v) => {
            return Err(ApplyError::IncompatibleVersion(v));
        }
    }

    let mut stage_params = Vec::new();
    for stage in &recipe.stages {
        if stage.enabled {
            stage_params.push((stage.stage_id.clone(), stage.params.clone()));
        }
    }

    Ok(stage_params)
}

pub fn integrity_label(badge: &IntegrityBadge) -> String {
    if badge.perceptual_models_used {
        "Perceptual AI used".into()
    } else if badge.deterministic_models_used {
        "Deterministic AI used".into()
    } else {
        "No AI models".into()
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ApplyError {
    #[error("Missing models: {0:?}")]
    MissingModels(Vec<String>),
    #[error("Incompatible version: {0}")]
    IncompatibleVersion(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_recipe_creation() {
        let recipe = Recipe::new("My Recipe", "deep_sky");
        assert_eq!(recipe.schema_version, "1.0");
        assert_eq!(recipe.name, "My Recipe");
        assert!(recipe.stages.is_empty());
    }

    #[test]
    fn test_add_stage_and_model() {
        let mut recipe = Recipe::new("Test", "deep_sky");
        let mut params = HashMap::new();
        params.insert("kappa".into(), serde_json::json!(3.0));
        recipe.add_stage("stacking", params);
        recipe.add_model("swinir-denoise-astro", ModelType::Deterministic);

        assert_eq!(recipe.stages.len(), 1);
        assert!(recipe.integrity.deterministic_models_used);
        assert!(!recipe.integrity.perceptual_models_used);
    }

    #[test]
    fn test_recipe_json_roundtrip() {
        let mut recipe = Recipe::new("Test", "deep_sky");
        recipe.add_stage("stretching", HashMap::new());
        let json = recipe.to_json().unwrap();
        let parsed = Recipe::from_json(&json).unwrap();
        assert_eq!(parsed.name, "Test");
        assert_eq!(parsed.stages.len(), 1);
    }

    #[test]
    fn test_sanitize_recipe_strips_paths() {
        let mut recipe = Recipe::new("Test", "deep_sky");
        let mut params = HashMap::new();
        params.insert("source_path".into(), serde_json::json!("/home/user/data"));
        params.insert("gps_lat".into(), serde_json::json!(51.5));
        params.insert("kappa".into(), serde_json::json!(3.0));
        recipe.add_stage("stacking", params);
        sanitize_recipe(&mut recipe);

        assert!(!recipe.stages[0].params.contains_key("source_path"));
        assert!(!recipe.stages[0].params.contains_key("gps_lat"));
        assert!(recipe.stages[0].params.contains_key("kappa"));
    }

    #[test]
    fn test_validate_compatibility_ok() {
        let recipe = Recipe::new("Test", "deep_sky");
        let available = vec!["swinir-denoise-astro".to_string()];
        assert_eq!(
            validate_compatibility(&recipe, &available),
            ValidationResult::Compatible
        );
    }

    #[test]
    fn test_validate_compatibility_missing_models() {
        let mut recipe = Recipe::new("Test", "deep_sky");
        recipe.add_model("swinir-denoise-astro", ModelType::Deterministic);
        let available = vec!["other-model".to_string()];
        match validate_compatibility(&recipe, &available) {
            ValidationResult::MissingModels(m) => {
                assert!(m.contains(&"swinir-denoise-astro".to_string()))
            }
            _ => panic!("Expected MissingModels"),
        }
    }

    #[test]
    fn test_apply_recipe() {
        let mut recipe = Recipe::new("Test", "deep_sky");
        let mut params = HashMap::new();
        params.insert("kappa".into(), serde_json::json!(3.0));
        recipe.add_stage("stacking", params);
        let available = vec![];
        let result = apply_recipe(&recipe, &available).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].0, "stacking");
    }

    #[test]
    fn test_integrity_label() {
        let badge = IntegrityBadge {
            perceptual_models_used: true,
            deterministic_models_used: false,
            seed_recorded: true,
            models: vec![],
        };
        assert_eq!(integrity_label(&badge), "Perceptual AI used");

        let badge = IntegrityBadge {
            perceptual_models_used: false,
            deterministic_models_used: false,
            seed_recorded: false,
            models: vec![],
        };
        assert_eq!(integrity_label(&badge), "No AI models");
    }
}
