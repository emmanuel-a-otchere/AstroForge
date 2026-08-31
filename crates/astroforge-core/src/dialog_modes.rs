use crate::image::F32Image;
use crate::mvp_pipeline::{DialogMode, Verbosity};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DialogState {
    pub mode: DialogMode,
    pub verbosity: Verbosity,
    pub stage_id: String,
    pub preview_before: Option<String>,
    pub preview_after: Option<String>,
    pub metrics: std::collections::HashMap<String, f64>,
    pub user_decision: Option<UserDecision>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum UserDecision {
    Accept,
    Adjust(std::collections::HashMap<String, serde_json::Value>),
    Skip,
    RevertToAuto,
}

pub fn create_confirm_dialog(
    stage_id: &str,
    preview_before: &str,
    preview_after: &str,
    metrics: std::collections::HashMap<String, f64>,
) -> DialogState {
    DialogState {
        mode: DialogMode::Confirm,
        verbosity: Verbosity::Intermediate,
        stage_id: stage_id.to_string(),
        preview_before: Some(preview_before.to_string()),
        preview_after: Some(preview_after.to_string()),
        metrics,
        user_decision: None,
    }
}

pub fn create_manual_dialog(
    stage_id: &str,
    default_params: std::collections::HashMap<String, serde_json::Value>,
) -> DialogState {
    DialogState {
        mode: DialogMode::Manual,
        verbosity: Verbosity::Expert,
        stage_id: stage_id.to_string(),
        preview_before: None,
        preview_after: None,
        metrics: std::collections::HashMap::new(),
        user_decision: Some(UserDecision::Adjust(default_params)),
    }
}

pub fn get_verbosity_params(verbosity: &Verbosity) -> Vec<&'static str> {
    match verbosity {
        Verbosity::Beginner => vec!["auto"],
        Verbosity::Intermediate => vec!["preview", "metrics", "ok_adjust_skip"],
        Verbosity::Expert => vec!["full_params", "histogram", "advanced_controls", "save_preset"],
    }
}

pub fn save_preset(name: &str, params: &std::collections::HashMap<String, serde_json::Value>) -> serde_json::Value {
    serde_json::json!({
        "name": name,
        "params": params,
        "version": "1.0",
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_confirm_dialog() {
        let metrics = std::collections::HashMap::from([("snr".into(), 42.0)]);
        let dialog = create_confirm_dialog("stacking", "before.png", "after.png", metrics);
        assert_eq!(dialog.mode, DialogMode::Confirm);
        assert_eq!(dialog.verbosity, Verbosity::Intermediate);
        assert!(dialog.preview_before.is_some());
        assert!(dialog.preview_after.is_some());
    }

    #[test]
    fn test_create_manual_dialog() {
        let params = std::collections::HashMap::from([("kappa".into(), serde_json::json!(3.0))]);
        let dialog = create_manual_dialog("stacking", params);
        assert_eq!(dialog.mode, DialogMode::Manual);
        assert_eq!(dialog.verbosity, Verbosity::Expert);
        assert!(matches!(dialog.user_decision, Some(UserDecision::Adjust(_))));
    }

    #[test]
    fn test_get_verbosity_params() {
        assert_eq!(get_verbosity_params(&Verbosity::Beginner), vec!["auto"]);
        assert!(get_verbosity_params(&Verbosity::Expert).contains(&"save_preset"));
    }

    #[test]
    fn test_save_preset() {
        let params = std::collections::HashMap::from([("kappa".into(), serde_json::json!(3.0))]);
        let preset = save_preset("my_preset", &params);
        assert_eq!(preset["name"], "my_preset");
        assert_eq!(preset["version"], "1.0");
    }
}
