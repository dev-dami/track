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
    assert!(build_res.is_ok(), "yard build failed: {:?}", build_res.err());
    assert!(proj_dir.join("target/multi_app").exists());
    assert!(proj_dir.join("target/.cache_meta.json").exists());

    // Second build (should hit cache)
    let rebuild_res = commands::build_at(&proj_dir, &[]);
    assert!(rebuild_res.is_ok(), "yard rebuild failed: {:?}", rebuild_res.err());

    let _ = fs::remove_dir_all(&temp_dir);
}
