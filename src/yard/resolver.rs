use super::manifest::{Dependency, Manifest};
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub struct ResolvedDep {
    pub name: String,
    pub version: String,
    pub source: DepSource,
    pub src_dir: PathBuf,
}

#[derive(Debug)]
pub enum DepSource {
    Local(PathBuf),
    Git { url: String, branch: Option<String> },
    Registry(String),
}

pub fn resolve(manifest: &Manifest, project_root: &Path) -> Result<Vec<ResolvedDep>, String> {
    let mut resolved = Vec::new();

    for (name, dep) in &manifest.dependencies {
        match dep {
            Dependency::Simple(version) => {
                // Registry dependency — not yet implemented
                eprintln!(
                    "  ⚠ Registry dependency '{}@{}' — registry not yet available, skipping",
                    name, version
                );
                resolved.push(ResolvedDep {
                    name: name.clone(),
                    version: version.clone(),
                    source: DepSource::Registry(version.clone()),
                    src_dir: PathBuf::new(),
                });
            }
            Dependency::Detailed {
                version,
                git,
                path,
                branch,
            } => {
                if let Some(local_path) = path {
                    // Local path dependency
                    let dep_path = project_root.join(local_path);
                    if !dep_path.exists() {
                        return Err(format!(
                            "Path dependency '{}' not found: {}",
                            name,
                            dep_path.display()
                        ));
                    }
                    let src_dir = dep_path.join("src");
                    resolved.push(ResolvedDep {
                        name: name.clone(),
                        version: version.clone().unwrap_or_else(|| "0.0.0".to_string()),
                        source: DepSource::Local(dep_path),
                        src_dir,
                    });
                } else if let Some(url) = git {
                    // Git dependency — not yet implemented
                    eprintln!(
                        "  ⚠ Git dependency '{}' ({}) — git fetch not yet available, skipping",
                        name, url
                    );
                    resolved.push(ResolvedDep {
                        name: name.clone(),
                        version: version.clone().unwrap_or_else(|| "0.0.0".to_string()),
                        source: DepSource::Git {
                            url: url.clone(),
                            branch: branch.clone(),
                        },
                        src_dir: PathBuf::new(),
                    });
                } else if let Some(ver) = version {
                    eprintln!(
                        "  ⚠ Registry dependency '{}@{}' — registry not yet available, skipping",
                        name, ver
                    );
                    resolved.push(ResolvedDep {
                        name: name.clone(),
                        version: ver.clone(),
                        source: DepSource::Registry(ver.clone()),
                        src_dir: PathBuf::new(),
                    });
                } else {
                    return Err(format!(
                        "Dependency '{}' must specify version, path, or git",
                        name
                    ));
                }
            }
        }
    }

    Ok(resolved)
}
