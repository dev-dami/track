use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process;

use super::builder::ParallelBuilder;
use super::manifest::{Dependency, Manifest};

// ── yard init ────────────────────────────────────────────────────────

pub fn init(args: &[String]) -> Result<(), String> {
    let name = args
        .first()
        .ok_or_else(|| "Usage: track yard init <project_name>".to_string())?;

    let project_dir = Path::new(name);
    if project_dir.exists() {
        return Err(format!("Directory '{}' already exists", name));
    }

    // Create project structure
    fs::create_dir_all(project_dir.join("src"))
        .map_err(|e| format!("Failed to create directory: {}", e))?;

    // Track.toml
    let pkg_name = project_dir
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| name.to_string());
    let manifest = Manifest::new(&pkg_name);
    manifest.save(&project_dir.join("Track.toml"))?;

    // src/main.trk
    fs::write(
        project_dir.join("src/main.trk"),
        "fn main() -> void {\n    print(42);\n}\n",
    )
    .map_err(|e| format!("Failed to write main.trk: {}", e))?;

    // .gitignore
    fs::write(project_dir.join(".gitignore"), "/target\n/yard.lock\n")
        .map_err(|e| format!("Failed to write .gitignore: {}", e))?;

    println!("✓ Created project '{}'", name);
    println!("  {}/Track.toml", name);
    println!("  {}/src/main.trk", name);
    println!("\nGet started:");
    println!("  cd {}", name);
    println!("  track yard build");
    println!("  track yard run");

    Ok(())
}

// ── yard build ───────────────────────────────────────────────────────

pub fn build(_args: &[String]) -> Result<(), String> {
    let project_root = find_project_root()?;
    build_at(&project_root, _args)
}

pub fn build_at(project_root: &Path, _args: &[String]) -> Result<(), String> {
    let manifest = Manifest::load(&project_root.join("Track.toml"))?;
    ParallelBuilder::build(project_root, &manifest)
}

// ── yard run ─────────────────────────────────────────────────────────

pub fn run_cmd(args: &[String]) -> Result<(), String> {
    let project_root = find_project_root()?;
    run_cmd_at(&project_root, args)
}

pub fn run_cmd_at(project_root: &Path, args: &[String]) -> Result<(), String> {
    build_at(project_root, args)?;

    let manifest = Manifest::load(&project_root.join("Track.toml"))?;
    let exe_path = project_root.join("target").join(&manifest.package.name);

    let status = process::Command::new(&exe_path)
        .status()
        .map_err(|e| format!("Failed to run '{}': {}", exe_path.display(), e))?;

    if !status.success() {
        return Err(format!("Process exited with code: {:?}", status.code()));
    }

    Ok(())
}

// ── yard add ─────────────────────────────────────────────────────────

pub fn add(args: &[String]) -> Result<(), String> {
    let project_root = find_project_root()?;
    add_at(&project_root, args)
}

pub fn add_at(project_root: &Path, args: &[String]) -> Result<(), String> {
    if args.is_empty() {
        return Err(
            "Usage: track yard add <package> [--version <ver>] [--path <path>] [--git <url>]"
                .to_string(),
        );
    }

    let pkg_name = &args[0];
    let manifest_path = project_root.join("Track.toml");
    let mut manifest = Manifest::load(&manifest_path)?;

    // Parse optional flags
    let mut version = None;
    let mut path = None;
    let mut git = None;
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--version" | "-v" => {
                i += 1;
                version = args.get(i).cloned();
            }
            "--path" | "-p" => {
                i += 1;
                path = args.get(i).cloned();
            }
            "--git" | "-g" => {
                i += 1;
                git = args.get(i).cloned();
            }
            _ => {}
        }
        i += 1;
    }

    let dep = if path.is_some() || git.is_some() {
        Dependency::Detailed {
            version,
            git,
            path,
            branch: None,
        }
    } else {
        Dependency::Simple(version.unwrap_or_else(|| "0.1.0".to_string()))
    };

    manifest.dependencies.insert(pkg_name.clone(), dep);
    manifest.save(&manifest_path)?;

    println!("✓ Added dependency '{}'", pkg_name);
    Ok(())
}

// ── yard check ───────────────────────────────────────────────────────

pub fn check(_args: &[String]) -> Result<(), String> {
    let project_root = find_project_root()?;
    check_at(&project_root, _args)
}

pub fn check_at(project_root: &Path, _args: &[String]) -> Result<(), String> {
    let manifest = Manifest::load(&project_root.join("Track.toml"))?;
    ParallelBuilder::check(project_root, &manifest)
}

// ── helpers ──────────────────────────────────────────────────────────

fn find_project_root() -> Result<PathBuf, String> {
    let mut dir =
        env::current_dir().map_err(|e| format!("Failed to get current directory: {}", e))?;

    loop {
        if dir.join("Track.toml").exists() {
            return Ok(dir);
        }
        if !dir.pop() {
            return Err(
                "No Track.toml found in current directory or any parent. Run 'track yard init <name>' to create a project."
                    .to_string(),
            );
        }
    }
}
