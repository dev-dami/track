use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

#[derive(Debug, Deserialize, Serialize)]
pub struct Manifest {
    pub package: Package,
    #[serde(default)]
    pub dependencies: HashMap<String, Dependency>,
    #[serde(default)]
    pub build: BuildConfig,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Package {
    pub name: String,
    pub version: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub authors: Vec<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(untagged)]
pub enum Dependency {
    Simple(String),
    Detailed {
        #[serde(default)]
        version: Option<String>,
        #[serde(default)]
        git: Option<String>,
        #[serde(default)]
        path: Option<String>,
        #[serde(default)]
        branch: Option<String>,
    },
}

#[derive(Debug, Deserialize, Serialize)]
pub struct BuildConfig {
    #[serde(default = "default_src")]
    pub src: String,
    #[serde(default)]
    pub target: Option<String>,
}

impl Default for BuildConfig {
    fn default() -> Self {
        Self {
            src: default_src(),
            target: None,
        }
    }
}

fn default_src() -> String {
    "src".to_string()
}

impl Manifest {
    pub fn load(path: &Path) -> Result<Self, String> {
        let content = fs::read_to_string(path)
            .map_err(|e| format!("Failed to read '{}': {}", path.display(), e))?;
        toml::from_str(&content).map_err(|e| format!("Failed to parse '{}': {}", path.display(), e))
    }

    pub fn save(&self, path: &Path) -> Result<(), String> {
        let content = toml::to_string_pretty(self)
            .map_err(|e| format!("Failed to serialize manifest: {}", e))?;
        fs::write(path, content).map_err(|e| format!("Failed to write '{}': {}", path.display(), e))
    }

    pub fn new(name: &str) -> Self {
        Self {
            package: Package {
                name: name.to_string(),
                version: "0.1.0".to_string(),
                description: None,
                authors: Vec::new(),
            },
            dependencies: HashMap::new(),
            build: BuildConfig::default(),
        }
    }
}
