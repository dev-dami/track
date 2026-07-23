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

    // Switch current dir into project for yard commands
    let orig_dir = env::current_dir().unwrap();
    env::set_current_dir(&proj_dir).unwrap();

    // Test yard check
    let check_res = commands::check(&[]);
    assert!(check_res.is_ok(), "yard check failed: {:?}", check_res.err());

    // Test yard add
    let add_res = commands::add(&["some_dep".to_string(), "--version".to_string(), "0.2.0".to_string()]);
    assert!(add_res.is_ok(), "yard add failed: {:?}", add_res.err());

    let updated_manifest = Manifest::load(&proj_dir.join("Track.toml")).unwrap();
    assert!(updated_manifest.dependencies.contains_key("some_dep"));

    // Cleanup & restore directory
    env::set_current_dir(&orig_dir).unwrap();
    let _ = fs::remove_dir_all(&temp_dir);
}
