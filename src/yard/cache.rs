use std::collections::HashMap;
use std::fs;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};

#[derive(Debug, serde::Serialize, serde::Deserialize, Default)]
pub struct BuildCache {
    pub entries: HashMap<String, String>, // relative path -> content hash
}

impl BuildCache {
    pub fn cache_file_path(target_dir: &Path) -> PathBuf {
        target_dir.join(".cache_meta.json")
    }

    pub fn load(target_dir: &Path) -> Self {
        let cache_path = Self::cache_file_path(target_dir);
        if let Ok(data) = fs::read_to_string(&cache_path) {
            if let Ok(cache) = serde_json::from_str(&data) {
                return cache;
            }
        }
        Self::default()
    }

    pub fn save(&self, target_dir: &Path) -> Result<(), String> {
        let cache_path = Self::cache_file_path(target_dir);
        let data = serde_json::to_string_pretty(self)
            .map_err(|e| format!("Failed to serialize build cache: {}", e))?;
        fs::write(&cache_path, data)
            .map_err(|e| format!("Failed to write build cache: {}", e))?;
        Ok(())
    }

    pub fn compute_hash(content: &str) -> String {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        content.hash(&mut hasher);
        format!("{:x}", hasher.finish())
    }

    pub fn is_cached(&self, rel_path: &str, content_hash: &str, obj_path: &Path) -> bool {
        if !obj_path.exists() {
            return false;
        }
        if let Some(cached_hash) = self.entries.get(rel_path) {
            cached_hash == content_hash
        } else {
            false
        }
    }

    pub fn update(&mut self, rel_path: String, content_hash: String) {
        self.entries.insert(rel_path, content_hash);
    }
}
