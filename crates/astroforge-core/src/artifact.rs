use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtifactId {
    pub session_id: String,
    pub stage_id: String,
    pub kind: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtifactRecord {
    pub id: ArtifactId,
    pub path: PathBuf,
    pub format: ArtifactFormat,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ArtifactFormat {
    Fits,
    Tiff,
    Png,
    Jpeg,
    Xisf,
    Json,
}

pub struct ArtifactStore {
    base_dir: PathBuf,
}

impl ArtifactStore {
    pub fn new(base_dir: PathBuf) -> Self {
        Self { base_dir }
    }

    pub fn path_for(&self, id: &ArtifactId) -> PathBuf {
        self.base_dir
            .join(&id.session_id)
            .join(&id.stage_id)
            .join(&id.kind)
    }

    pub fn save(
        &self,
        id: &ArtifactId,
        data: &[u8],
        format: ArtifactFormat,
    ) -> Result<PathBuf, std::io::Error> {
        let path = self.path_for(id);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&path, data)?;
        Ok(path)
    }

    pub fn load(&self, id: &ArtifactId) -> Result<Vec<u8>, std::io::Error> {
        let path = self.path_for(id);
        std::fs::read(&path)
    }

    pub fn exists(&self, id: &ArtifactId) -> bool {
        self.path_for(id).exists()
    }
}
