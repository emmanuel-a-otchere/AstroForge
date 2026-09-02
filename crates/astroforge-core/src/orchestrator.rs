use crate::pipeline::{PipelineDag, Stage, StageContext, StageResult};
use std::collections::HashMap;
use std::sync::Arc;

pub struct Orchestrator {
    dag: PipelineDag,
    stages: HashMap<String, Arc<dyn Stage>>,
    state: OrchestratorState,
}

#[derive(Debug, Clone, PartialEq)]
pub enum OrchestratorState {
    Idle,
    Running { current_stage: String },
    Paused { current_stage: String },
    Completed,
    Failed { stage_id: String, error: String },
}

impl Orchestrator {
    pub fn new(dag: PipelineDag) -> Self {
        Self {
            dag,
            stages: HashMap::new(),
            state: OrchestratorState::Idle,
        }
    }

    pub fn register_stage(&mut self, stage: Arc<dyn Stage>) {
        let id = stage.id().to_string();
        self.stages.insert(id, stage);
    }

    pub fn run(&mut self, params: HashMap<String, serde_json::Value>) -> Vec<StageResult> {
        let order = match self.dag.topological_order() {
            Ok(o) => o,
            Err(e) => {
                self.state = OrchestratorState::Failed {
                    stage_id: "dag".into(),
                    error: e.to_string(),
                };
                return vec![];
            }
        };

        let mut results = Vec::new();
        for stage_id in &order {
            let stage = match self.stages.get(stage_id) {
                Some(s) => s,
                None => continue,
            };

            self.state = OrchestratorState::Running {
                current_stage: stage_id.clone(),
            };

            let ctx = StageContext {
                stage_id: stage_id.clone(),
                params: params.clone(),
            };

            match stage.run(&ctx) {
                Ok(result) => {
                    if result.success {
                        results.push(result);
                    } else {
                        self.state = OrchestratorState::Failed {
                            stage_id: stage_id.clone(),
                            error: result.error.clone().unwrap_or_default(),
                        };
                        results.push(result);
                        return results;
                    }
                }
                Err(e) => {
                    self.state = OrchestratorState::Failed {
                        stage_id: stage_id.clone(),
                        error: e.to_string(),
                    };
                    return results;
                }
            }
        }

        self.state = OrchestratorState::Completed;
        results
    }

    pub fn state(&self) -> &OrchestratorState {
        &self.state
    }
}
