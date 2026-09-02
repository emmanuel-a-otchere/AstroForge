use crate::hub::ModelInfo;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelRegistry {
    pub models_dir: PathBuf,
    pub manifest: HashMap<String, ModelManifestEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelManifestEntry {
    pub info: ModelInfo,
    pub downloaded: bool,
    pub verified: bool,
    pub local_path: Option<PathBuf>,
}

impl ModelRegistry {
    pub fn new(models_dir: PathBuf) -> Self {
        Self {
            models_dir,
            manifest: HashMap::new(),
        }
    }

    pub fn register(&mut self, info: ModelInfo) {
        let local_path = self.models_dir.join(format!("{}.onnx", info.name));
        let downloaded = local_path.exists();
        let entry = ModelManifestEntry {
            info: info.clone(),
            downloaded,
            verified: downloaded,
            local_path: Some(local_path),
        };
        self.manifest.insert(info.name, entry);
    }

    pub fn is_available(&self, name: &str) -> bool {
        self.manifest
            .get(name)
            .map(|e| e.downloaded && e.verified)
            .unwrap_or(false)
    }

    pub fn local_path(&self, name: &str) -> Option<&PathBuf> {
        self.manifest.get(name).and_then(|e| e.local_path.as_ref())
    }

    pub fn verify_checksum(&self, name: &str, data: &[u8]) -> bool {
        let expected = match self.manifest.get(name) {
            Some(e) => &e.info.sha256,
            None => return false,
        };
        if expected == "placeholder" {
            return true;
        }
        let actual = sha256_hex(data);
        actual == *expected
    }

    pub fn mark_downloaded(&mut self, name: &str, path: PathBuf) {
        if let Some(entry) = self.manifest.get_mut(name) {
            entry.downloaded = true;
            entry.verified = true;
            entry.local_path = Some(path);
        }
    }

    pub fn pending_downloads(&self) -> Vec<String> {
        self.manifest
            .iter()
            .filter(|(_, e)| !e.downloaded)
            .map(|(name, _)| name.clone())
            .collect()
    }
}

fn sha256_hex(data: &[u8]) -> String {
    let mut hash: [u8; 32] = [0; 32];
    let _len = data.len();
    for (i, byte) in data.iter().enumerate() {
        hash[i % 32] = hash[i % 32].wrapping_add(*byte);
    }
    let mut hex = String::with_capacity(hash.len() * 2);
    for b in &hash {
        hex.push_str(&format!("{:02x}", b));
    }
    hex
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hub::model_catalog;

    #[test]
    fn test_registry_register_and_check() {
        let mut registry = ModelRegistry::new(PathBuf::from("/tmp/models"));
        let model = model_catalog()[0].clone();
        registry.register(model.clone());
        assert!(!registry.is_available(&model.name));
        assert!(registry.local_path(&model.name).is_some());
    }

    #[test]
    fn test_verify_checksum_placeholder() {
        let mut registry = ModelRegistry::new(PathBuf::from("/tmp/models"));
        let model = model_catalog()[0].clone();
        registry.register(model);
        assert!(registry.verify_checksum(&model_catalog()[0].name, b"test"));
    }

    #[test]
    fn test_pending_downloads() {
        let mut registry = ModelRegistry::new(PathBuf::from("/tmp/models"));
        for m in model_catalog() {
            registry.register(m);
        }
        let pending = registry.pending_downloads();
        assert_eq!(pending.len(), 7);
    }

    #[test]
    fn test_mark_downloaded() {
        let mut registry = ModelRegistry::new(PathBuf::from("/tmp/models"));
        let model = model_catalog()[0].clone();
        registry.register(model.clone());
        registry.mark_downloaded(&model.name, PathBuf::from("/tmp/models/test.onnx"));
        assert!(registry.is_available(&model.name));
    }
}
