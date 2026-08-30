use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StageContext {
    pub stage_id: String,
    pub params: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StageResult {
    pub stage_id: String,
    pub success: bool,
    pub metrics: HashMap<String, f64>,
    pub error: Option<String>,
}

pub trait Stage: Send + Sync {
    fn id(&self) -> &str;
    fn run(&self, ctx: &StageContext) -> Result<StageResult, StageError>;
}

#[derive(Debug, thiserror::Error)]
pub enum StageError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Serialization error: {0}")]
    Serde(#[from] serde_json::Error),
    #[error("Stage failed: {0}")]
    Failed(String),
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PipelineDag {
    pub stages: Vec<String>,
    pub edges: Vec<(String, String)>,
}

impl PipelineDag {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_stage(&mut self, id: String) {
        if !self.stages.contains(&id) {
            self.stages.push(id);
        }
    }

    pub fn add_edge(&mut self, from: String, to: String) {
        self.edges.push((from, to));
    }
}
