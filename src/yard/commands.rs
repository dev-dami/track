use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process;

use super::manifest::{Dependency, Manifest};
use super::resolver;

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
    let manifest = Manifest::new(name);
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
    let manifest = Manifest::load(&project_root.join("Track.toml"))?;

    println!(
        "  Building {} v{}",
        manifest.package.name, manifest.package.version
    );

    // Resolve dependencies
    let deps = resolver::resolve(&manifest, &project_root)?;
    for dep in &deps {
        match &dep.source {
            resolver::DepSource::Local(path) => {
                println!(
                    "  Using dependency: {} v{} (local path: {})",
                    dep.name,
                    dep.version,
                    path.display()
                );
            }
            resolver::DepSource::Git { url, branch } => {
                println!(
                    "  Using dependency: {} v{} (git: {}, branch: {:?})",
                    dep.name, dep.version, url, branch
                );
            }
            resolver::DepSource::Registry(version) => {
                println!(
                    "  Using dependency: {} v{} (registry version: {})",
                    dep.name, dep.version, version
                );
            }
        }
        let _ = &dep.src_dir;
    }

    // Find all .trk source files
    let src_dir = project_root.join(&manifest.build.src);
    if !src_dir.exists() {
        return Err(format!(
            "Source directory '{}' not found",
            src_dir.display()
        ));
    }

    let trk_files = find_trk_files(&src_dir)?;
    if trk_files.is_empty() {
        return Err("No .trk source files found".to_string());
    }

    // Create target directory
    let target_dir = project_root.join("target");
    fs::create_dir_all(&target_dir)
        .map_err(|e| format!("Failed to create target directory: {}", e))?;

    // Compile each file through the full pipeline
    let mut obj_files = Vec::new();

    for trk_file in &trk_files {
        let source = fs::read_to_string(trk_file)
            .map_err(|e| format!("Failed to read '{}': {}", trk_file.display(), e))?;

        // Lex
        let tokens = crate::lexer::Lexer::tokenize(&source)
            .map_err(|e| format!("{}: {}", trk_file.display(), e))?;

        // Parse
        let mut parser = crate::parser::Parser::new(tokens, source.clone());
        let program = parser
            .parse_program()
            .map_err(|e| format!("{}: {}", trk_file.display(), e))?;

        // Linear check
        let mut checker = crate::checker::LinearChecker::new();
        checker
            .check_program(&program)
            .map_err(|e| format!("{}: {}", trk_file.display(), e))?;

        // Codegen
        let context = inkwell::context::Context::create();
        let mut codegen = crate::codegen::CodeGen::new(&context, "track_module");
        codegen.compile_program(&program);

        // Emit object file
        let stem = trk_file
            .file_stem()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        let obj_path = target_dir.join(format!("{}.o", stem));

        codegen.write_object_file(&obj_path)?;
        obj_files.push(obj_path);
    }

    // Link all object files into final binary
    let exe_name = &manifest.package.name;
    let exe_path = target_dir.join(exe_name);

    let mut cmd = process::Command::new("cc");
    for obj in &obj_files {
        cmd.arg(obj);
    }
    cmd.arg("-o").arg(&exe_path).arg("-lm").arg("-no-pie");

    let status = cmd.status().map_err(|e| format!("Linker failed: {}", e))?;

    if !status.success() {
        return Err(format!("Linker failed with exit code: {:?}", status.code()));
    }

    // Clean up object files
    for obj in &obj_files {
        let _ = fs::remove_file(obj);
    }

    let file_count = trk_files.len();
    println!(
        "✓ Built {} ({} file{})",
        exe_path.display(),
        file_count,
        if file_count == 1 { "" } else { "s" }
    );

    Ok(())
}

// ── yard run ─────────────────────────────────────────────────────────

pub fn run_cmd(args: &[String]) -> Result<(), String> {
    build(args)?;

    let project_root = find_project_root()?;
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
    if args.is_empty() {
        return Err(
            "Usage: track yard add <package> [--version <ver>] [--path <path>] [--git <url>]"
                .to_string(),
        );
    }

    let pkg_name = &args[0];
    let project_root = find_project_root()?;
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
    let manifest = Manifest::load(&project_root.join("Track.toml"))?;

    println!(
        "  Checking {} v{}",
        manifest.package.name, manifest.package.version
    );

    let src_dir = project_root.join(&manifest.build.src);
    if !src_dir.exists() {
        return Err(format!(
            "Source directory '{}' not found",
            src_dir.display()
        ));
    }

    let trk_files = find_trk_files(&src_dir)?;
    if trk_files.is_empty() {
        return Err("No .trk source files found".to_string());
    }

    let mut errors = Vec::new();

    for trk_file in &trk_files {
        let source = fs::read_to_string(trk_file)
            .map_err(|e| format!("Failed to read '{}': {}", trk_file.display(), e))?;

        // Lex
        let tokens = match crate::lexer::Lexer::tokenize(&source) {
            Ok(t) => t,
            Err(e) => {
                errors.push(format!("{}: {}", trk_file.display(), e));
                continue;
            }
        };

        // Parse
        let mut parser = crate::parser::Parser::new(tokens, source.clone());
        let program = match parser.parse_program() {
            Ok(p) => p,
            Err(e) => {
                errors.push(format!("{}: {}", trk_file.display(), e));
                continue;
            }
        };

        // Linear check
        let mut checker = crate::checker::LinearChecker::new();
        if let Err(e) = checker.check_program(&program) {
            errors.push(format!("{}: {}", trk_file.display(), e));
        }
    }

    if errors.is_empty() {
        println!(
            "✓ Check passed ({} file{})",
            trk_files.len(),
            if trk_files.len() == 1 { "" } else { "s" }
        );
        Ok(())
    } else {
        for e in &errors {
            eprintln!("  ✗ {}", e);
        }
        Err(format!("{} error(s) found", errors.len()))
    }
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

fn find_trk_files(dir: &Path) -> Result<Vec<PathBuf>, String> {
    let mut files = Vec::new();
    collect_trk_files(dir, &mut files)?;
    files.sort();
    Ok(files)
}

fn collect_trk_files(dir: &Path, out: &mut Vec<PathBuf>) -> Result<(), String> {
    let entries = fs::read_dir(dir)
        .map_err(|e| format!("Failed to read directory '{}': {}", dir.display(), e))?;

    for entry in entries {
        let entry = entry.map_err(|e| format!("Directory entry error: {}", e))?;
        let path = entry.path();

        if path.is_dir() {
            collect_trk_files(&path, out)?;
        } else if path.extension().is_some_and(|ext| ext == "trk") {
            out.push(path);
        }
    }

    Ok(())
}
