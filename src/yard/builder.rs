use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::thread;

use super::cache::BuildCache;
use super::linker::LinkerEngine;
use super::manifest::Manifest;
use super::resolver;

pub struct ParallelBuilder;

struct BuildTask {
    trk_file: PathBuf,
    rel_path: String,
    source: String,
    hash: String,
    obj_path: PathBuf,
    is_cached: bool,
}

impl ParallelBuilder {
    pub fn build(project_root: &Path, manifest: &Manifest) -> Result<(), String> {
        println!(
            "  Building {} v{}",
            manifest.package.name, manifest.package.version
        );

        // Resolve dependencies
        let deps = resolver::resolve(manifest, project_root)?;
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
        }

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

        let target_dir = project_root.join("target");
        fs::create_dir_all(&target_dir)
            .map_err(|e| format!("Failed to create target directory: {}", e))?;

        let mut cache = BuildCache::load(&target_dir);

        // Prepare build tasks
        let mut tasks = Vec::new();
        for trk_file in trk_files {
            let rel_path = trk_file
                .strip_prefix(project_root)
                .unwrap_or(&trk_file)
                .to_string_lossy()
                .to_string();

            let source = fs::read_to_string(&trk_file)
                .map_err(|e| format!("Failed to read '{}': {}", trk_file.display(), e))?;

            let stem = trk_file
                .file_stem()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();
            let obj_path = target_dir.join(format!("{}.o", stem));

            let (hash, is_cached) = if !obj_path.exists() {
                let hash = BuildCache::compute_hash(&source);
                (hash, false)
            } else {
                let hash = BuildCache::compute_hash(&source);
                let cached = cache.is_cached(&rel_path, &hash, &obj_path);
                (hash, cached)
            };

            tasks.push(BuildTask {
                trk_file,
                rel_path,
                source,
                hash,
                obj_path,
                is_cached,
            });
        }

        let total_files = tasks.len();
        let cached_count = tasks.iter().filter(|t| t.is_cached).count();

        let num_workers = thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(4);

        let shared_tasks = Arc::new(Mutex::new(
            tasks.into_iter().enumerate().collect::<Vec<_>>(),
        ));
        let results = Arc::new(Mutex::new(Vec::new()));

        let mut handles = Vec::new();

        let target_isa = crate::codegen::CodeGen::create_default_isa();
        for _ in 0..num_workers {
            let shared_tasks = Arc::clone(&shared_tasks);
            let results = Arc::clone(&results);
            let target_isa = Arc::clone(&target_isa);

            let handle = thread::spawn(move || {
                loop {
                    let task_option = {
                        let mut lock = shared_tasks.lock().unwrap();
                        lock.pop()
                    };

                    let (_idx, task) = match task_option {
                        Some(t) => t,
                        None => break,
                    };

                    if task.is_cached {
                        results
                            .lock()
                            .unwrap()
                            .push(Ok((task.rel_path, task.hash, task.obj_path)));
                        continue;
                    }

                    // Compile file: Lex -> Parse -> LinearCheck -> LLVM Codegen
                    let res = (|| -> Result<(String, String, PathBuf), String> {
                        let tokens = crate::lexer::Lexer::tokenize(&task.source)
                            .map_err(|e| format!("{}: {}", task.trk_file.display(), e))?;

                        let mut parser = crate::parser::Parser::new(tokens, task.source.clone());
                        let mut program = parser
                            .parse_program()
                            .map_err(|e| format!("{}: {}", task.trk_file.display(), e))?;

                        crate::mono::monomorphize(&mut program)
                            .map_err(|e| format!("{}: {}", task.trk_file.display(), e))?;

                        let mut checker = crate::checker::LinearChecker::new();
                        checker
                            .check_program(&program)
                            .map_err(|e| format!("{}: {}", task.trk_file.display(), e))?;

                        let mod_name = task
                            .trk_file
                            .file_stem()
                            .and_then(|s| s.to_str())
                            .unwrap_or("module");
                        let mut codegen =
                            crate::codegen::CodeGen::new_with_isa(mod_name, target_isa.clone());
                        codegen.compile_program(&program);

                        codegen.write_object_file(&task.obj_path)?;

                        Ok((task.rel_path, task.hash, task.obj_path))
                    })();

                    results.lock().unwrap().push(res);
                }
            });

            handles.push(handle);
        }

        for handle in handles {
            if handle.join().is_err() {
                return Err("Parallel builder worker thread panicked during execution".to_string());
            }
        }

        let mut obj_files = Vec::new();
        let compiled_results = match Arc::try_unwrap(results) {
            Ok(mutex) => mutex.into_inner().unwrap_or_default(),
            Err(arc) => arc.lock().unwrap().clone(),
        };

        for res in compiled_results {
            match res {
                Ok((rel_path, hash, obj_path)) => {
                    cache.update(rel_path, hash);
                    obj_files.push(obj_path);
                }
                Err(e) => return Err(e),
            }
        }

        // Save build cache metadata
        let _ = cache.save(&target_dir);

        // Sort obj_files deterministically for reproducible links
        obj_files.sort();

        let exe_name = Path::new(&manifest.package.name)
            .file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| manifest.package.name.clone());
        let exe_path = target_dir.join(exe_name);

        LinkerEngine::link_binary(&obj_files, &exe_path, &target_dir)?;

        println!(
            "✓ Built {} ({} file{}, {} cached)",
            exe_path.display(),
            total_files,
            if total_files == 1 { "" } else { "s" },
            cached_count
        );

        Ok(())
    }

    pub fn check(project_root: &Path, manifest: &Manifest) -> Result<(), String> {
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

        let num_workers = thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(4);

        let shared_files = Arc::new(Mutex::new(trk_files.clone()));
        let errors = Arc::new(Mutex::new(Vec::new()));

        let mut handles = Vec::new();

        for _ in 0..num_workers {
            let shared_files = Arc::clone(&shared_files);
            let errors = Arc::clone(&errors);

            let handle = thread::spawn(move || {
                loop {
                    let file_option = {
                        let mut lock = shared_files.lock().unwrap();
                        lock.pop()
                    };

                    let trk_file = match file_option {
                        Some(f) => f,
                        None => break,
                    };

                    let source = match fs::read_to_string(&trk_file) {
                        Ok(s) => s,
                        Err(e) => {
                            errors
                                .lock()
                                .unwrap()
                                .push(format!("{}: {}", trk_file.display(), e));
                            continue;
                        }
                    };

                    let tokens = match crate::lexer::Lexer::tokenize(&source) {
                        Ok(t) => t,
                        Err(e) => {
                            errors
                                .lock()
                                .unwrap()
                                .push(format!("{}: {}", trk_file.display(), e));
                            continue;
                        }
                    };

                    let mut parser = crate::parser::Parser::new(tokens, source.clone());
                    let mut program = match parser.parse_program() {
                        Ok(p) => p,
                        Err(e) => {
                            errors
                                .lock()
                                .unwrap()
                                .push(format!("{}: {}", trk_file.display(), e));
                            continue;
                        }
                    };

                    if let Err(e) = crate::mono::monomorphize(&mut program) {
                        errors
                            .lock()
                            .unwrap()
                            .push(format!("{}: {}", trk_file.display(), e));
                        continue;
                    }

                    let mut checker = crate::checker::LinearChecker::new();
                    if let Err(e) = checker.check_program(&program) {
                        errors
                            .lock()
                            .unwrap()
                            .push(format!("{}: {}", trk_file.display(), e));
                    }
                }
            });

            handles.push(handle);
        }

        for handle in handles {
            let _ = handle.join();
        }

        let errs = Arc::try_unwrap(errors).unwrap().into_inner().unwrap();

        if errs.is_empty() {
            println!(
                "✓ Check passed ({} file{})",
                trk_files.len(),
                if trk_files.len() == 1 { "" } else { "s" }
            );
            Ok(())
        } else {
            for e in &errs {
                eprintln!("  ✗ {}", e);
            }
            Err(format!("{} error(s) found", errs.len()))
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
