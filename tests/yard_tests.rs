use std::env;
use std::fs;
use track::yard::commands;
use track::yard::manifest::Manifest;

#[test]
fn test_yard_manifest_serialization() {
    let mut manifest = Manifest::new("test_proj");
    manifest.package.version = "1.2.3".to_string();
    let toml_str = toml::to_string_pretty(&manifest).unwrap();
    assert!(toml_str.contains("name = \"test_proj\""));
    assert!(toml_str.contains("version = \"1.2.3\""));

    let loaded: Manifest = toml::from_str(&toml_str).unwrap();
    assert_eq!(loaded.package.name, "test_proj");
    assert_eq!(loaded.package.version, "1.2.3");
}

#[test]
fn test_yard_init_add_check_in_temp_dir() {
    let temp_dir = env::temp_dir().join(format!("track_test_yard_{}", std::process::id()));
    let _ = fs::remove_dir_all(&temp_dir);

    let proj_dir = temp_dir.join("my_app");

    // Test yard init
    let init_res = commands::init(&[proj_dir.to_str().unwrap().to_string()]);
    assert!(init_res.is_ok(), "yard init failed: {:?}", init_res.err());

    assert!(proj_dir.join("Track.toml").exists());
    assert!(proj_dir.join("src/main.trk").exists());
    assert!(proj_dir.join(".gitignore").exists());

    // Test yard check_at
    let check_res = commands::check_at(&proj_dir, &[]);
    assert!(
        check_res.is_ok(),
        "yard check failed: {:?}",
        check_res.err()
    );

    // Test yard add_at
    let add_res = commands::add_at(
        &proj_dir,
        &[
            "some_dep".to_string(),
            "--version".to_string(),
            "0.2.0".to_string(),
        ],
    );
    assert!(add_res.is_ok(), "yard add failed: {:?}", add_res.err());

    let updated_manifest = Manifest::load(&proj_dir.join("Track.toml")).unwrap();
    assert!(updated_manifest.dependencies.contains_key("some_dep"));

    // Cleanup
    let _ = fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_yard_parallel_build_and_incremental_cache() {
    let temp_dir = env::temp_dir().join(format!("track_test_parallel_{}", std::process::id()));
    let _ = fs::remove_dir_all(&temp_dir);

    let proj_dir = temp_dir.join("multi_app");

    // Init project
    let init_res = commands::init(&[proj_dir.to_str().unwrap().to_string()]);
    assert!(init_res.is_ok(), "yard init failed: {:?}", init_res.err());

    // Add extra source files to test parallel building
    fs::write(
        proj_dir.join("src/helper1.trk"),
        "fn helper1() -> i32 { return 10; }\n",
    )
    .unwrap();

    fs::write(
        proj_dir.join("src/helper2.trk"),
        "fn helper2() -> i32 { return 20; }\n",
    )
    .unwrap();

    // Initial parallel build
    let build_res = commands::build_at(&proj_dir, &[]);
    assert!(
        build_res.is_ok(),
        "yard build failed: {:?}",
        build_res.err()
    );
    assert!(proj_dir.join("target/multi_app").exists());
    assert!(proj_dir.join("target/.cache_meta.json").exists());

    // Second build (should hit cache)
    let rebuild_res = commands::build_at(&proj_dir, &[]);
    assert!(
        rebuild_res.is_ok(),
        "yard rebuild failed: {:?}",
        rebuild_res.err()
    );

    let _ = fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_yard_lint_and_clean() {
    let td = env::temp_dir().join(format!("track_yard_lint_{}", std::process::id()));
    let _ = fs::remove_dir_all(&td);
    let proj = td.join("lint_app");
    commands::init(&[proj.to_str().unwrap().to_string()]).unwrap();
    // lint should succeed on fresh project
    let lint = commands::lint_at(&proj, &[]);
    assert!(lint.is_ok(), "lint failed: {:?}", lint.err());
    // build then clean
    let build = commands::build_at(&proj, &[]);
    assert!(build.is_ok(), "build failed: {:?}", build.err());
    assert!(proj.join("target").exists());
    let clean = commands::clean_at(&proj);
    assert!(clean.is_ok());
    assert!(!proj.join("target").exists());
    let _ = fs::remove_dir_all(&td);
}

#[test]
fn test_yard_version_reports() {
    let out = std::process::Command::new(env!("CARGO_BIN_EXE_yard"))
        .arg("--version")
        .output();
    if let Ok(o) = out {
        let s = format!(
            "{}{}",
            String::from_utf8_lossy(&o.stdout),
            String::from_utf8_lossy(&o.stderr)
        );
        assert!(s.contains("0.7.0"), "yard version missing 0.7.0: {}", s);
    } else {
        // fallback via cargo run
        let o = std::process::Command::new("cargo")
            .args(["run", "--bin", "yard", "--", "--version"])
            .output()
            .unwrap();
        let s = format!(
            "{}{}",
            String::from_utf8_lossy(&o.stdout),
            String::from_utf8_lossy(&o.stderr)
        );
        assert!(s.contains("0.7.0"), "yard version missing: {}", s);
    }
}

#[test]
fn test_yard_build_generics_project() {
    let td = env::temp_dir().join(format!("track_yard_gen_{}", std::process::id()));
    let _ = fs::remove_dir_all(&td);
    let proj = td.join("gen_app");
    commands::init(&[proj.to_str().unwrap().to_string()]).unwrap();
    fs::write(
        proj.join("src/main.trk"),
        "fn identity<T>(x: T) -> T { return x; } fn main() -> void { let a: i32 = 9; print(identity(a)); }",
    ).unwrap();
    let build = commands::build_at(&proj, &[]);
    assert!(build.is_ok(), "generics build failed: {:?}", build.err());
    assert!(proj.join("target/gen_app").exists());
    let check = commands::check_at(&proj, &[]);
    assert!(check.is_ok(), "generics check failed: {:?}", check.err());
    let lint = commands::lint_at(&proj, &[]);
    assert!(lint.is_ok());
    let _ = fs::remove_dir_all(&td);
}

#[test]
fn test_yard_nested_modules_have_isolated_stable_artifacts() {
    let root = env::temp_dir().join(format!("track_yard_artifacts_{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    let manifest = Manifest::new("artifacts");
    fs::create_dir_all(root.join("src/left")).unwrap();
    fs::create_dir_all(root.join("src/right")).unwrap();
    for (path, source) in [
        ("main.trk", "fn main() -> void { print(42); }"),
        ("left/helper.trk", "fn left_helper() -> i32 { return 10; }"),
        (
            "right/helper.trk",
            "fn right_helper() -> i32 { return 20; }",
        ),
        (
            "_track_runtime.trk",
            "fn user_runtime() -> i32 { return 30; }",
        ),
    ] {
        fs::write(root.join("src").join(path), source).unwrap();
    }

    let build = || track::yard::builder::ParallelBuilder::build(&root, &manifest).unwrap();
    build();
    let objects: Vec<_> = [
        "main.o",
        "left/helper.o",
        "right/helper.o",
        "_track_runtime.o",
    ]
    .iter()
    .map(|name| root.join("target/objects").join(name))
    .collect();
    let original_objects: Vec<_> = objects.iter().map(|path| fs::read(path).unwrap()).collect();
    assert_ne!(original_objects[1], original_objects[2]);
    assert!(root.join("target/_track_runtime.o").exists());
    let metadata_path = root.join("target/.cache_meta.json");
    let metadata = fs::read(&metadata_path).unwrap();

    // Warm builds must preserve objects as well as serialized metadata.
    let modified: Vec<_> = objects
        .iter()
        .map(|p| fs::metadata(p).unwrap().modified().unwrap())
        .collect();
    build();
    assert_eq!(metadata, fs::read(&metadata_path).unwrap());
    for (path, before) in objects.iter().zip(modified) {
        assert_eq!(fs::metadata(path).unwrap().modified().unwrap(), before);
    }

    // A clean build must recreate the same artifact names and metadata.
    fs::remove_dir_all(root.join("target")).unwrap();
    build();
    assert_eq!(metadata, fs::read(&metadata_path).unwrap());
    for path in &objects {
        assert!(path.is_file(), "missing {}", path.display());
    }
    let output = std::process::Command::new(root.join("target/artifacts"))
        .output()
        .unwrap();
    assert!(output.status.success());
    assert_eq!(String::from_utf8(output.stdout).unwrap().trim(), "42");
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn test_yard_build_errors_follow_source_order() {
    let root = env::temp_dir().join(format!("track_yard_error_order_{}", std::process::id()));
    fs::create_dir_all(root.join("src")).unwrap();
    for name in ["z.trk", "a.trk"] {
        fs::write(root.join("src").join(name), "fn invalid( {").unwrap();
    }
    let manifest = Manifest::new("errors");
    for _ in 0..4 {
        let error = track::yard::builder::ParallelBuilder::build(&root, &manifest).unwrap_err();
        assert!(error.contains("a.trk"), "unexpected first error: {error}");
    }
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn test_yard_dependency_resolution_is_sorted() {
    use track::yard::{manifest::Dependency, resolver};
    let mut manifest = Manifest::new("ordered");
    for name in ["zebra", "alpha", "middle"] {
        manifest
            .dependencies
            .insert(name.into(), Dependency::Simple("1.0".into()));
    }
    let dependencies = resolver::resolve(&manifest, std::path::Path::new(".")).unwrap();
    assert_eq!(
        dependencies
            .iter()
            .map(|dep| dep.name.as_str())
            .collect::<Vec<_>>(),
        ["alpha", "middle", "zebra"]
    );
}
