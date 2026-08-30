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

    pub fn topological_order(&self) -> Result<Vec<String>, StageError> {
        let mut in_degree: HashMap<String, usize> = HashMap::new();
        for s in &self.stages {
            in_degree.insert(s.clone(), 0);
        }
        for (_, to) in &self.edges {
            if let Some(d) = in_degree.get_mut(to) {
                *d += 1;
            }
        }
        let mut queue: Vec<String> = self
            .stages
            .iter()
            .filter(|s| *in_degree.get(*s).unwrap_or(&0) == 0)
            .cloned()
            .collect();
        let mut result = Vec::new();
        while let Some(node) = queue.pop() {
            result.push(node.clone());
            for (from, to) in &self.edges {
                if from == &node {
                    if let Some(d) = in_degree.get_mut(to) {
                        *d -= 1;
                        if *d == 0 {
                            queue.push(to.clone());
                        }
                    }
                }
            }
        }
        if result.len() != self.stages.len() {
            return Err(StageError::Failed("Cycle detected in DAG".into()));
        }
        Ok(result)
    }
}
