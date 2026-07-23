use serde_json::Value;
use std::fs;
use std::path::Path;
use std::process::Command;

#[test]
fn test_vscode_package_json() {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".to_string());
    let pkg_path = Path::new(&manifest_dir).join("editor/vscode/package.json");

    assert!(
        pkg_path.exists(),
        "package.json does not exist at editor/vscode/package.json"
    );

    let content = fs::read_to_string(&pkg_path).expect("Failed to read package.json");
    let v: Value = serde_json::from_str(&content).expect("package.json is invalid JSON");

    assert_eq!(v["name"], "track-vscode");
    assert_eq!(v["publisher"], "dev-dami");
    assert_eq!(v["icon"], "../../assets/track-icon.svg");

    let contributes = &v["contributes"];
    let languages = contributes["languages"]
        .as_array()
        .expect("languages array missing");
    assert!(!languages.is_empty());
    assert_eq!(languages[0]["id"], "track");
}

#[test]
fn test_vscode_language_configuration() {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".to_string());
    let config_path = Path::new(&manifest_dir).join("editor/vscode/language-configuration.json");

    assert!(
        config_path.exists(),
        "language-configuration.json does not exist"
    );

    let content =
        fs::read_to_string(&config_path).expect("Failed to read language-configuration.json");
    let v: Value =
        serde_json::from_str(&content).expect("language-configuration.json is invalid JSON");

    assert_eq!(v["comments"]["lineComment"], "//");
    assert!(v["brackets"].is_array());
    assert!(v["autoClosingPairs"].is_array());
}

#[test]
fn test_vscode_textmate_grammar() {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".to_string());
    let grammar_path =
        Path::new(&manifest_dir).join("editor/vscode/syntaxes/track.tmLanguage.json");

    assert!(
        grammar_path.exists(),
        "track.tmLanguage.json does not exist"
    );

    let content = fs::read_to_string(&grammar_path).expect("Failed to read tmLanguage.json");
    let v: Value = serde_json::from_str(&content).expect("tmLanguage.json is invalid JSON");

    assert_eq!(v["scopeName"], "source.track");
    assert_eq!(v["name"], "Track");

    let keywords_match = v["repository"]["keywords"]["patterns"][0]["match"]
        .as_str()
        .expect("keywords match pattern missing");

    assert!(
        keywords_match.contains("import"),
        "Grammar keywords missing 'import'"
    );
    assert!(
        keywords_match.contains("fn"),
        "Grammar keywords missing 'fn'"
    );
    assert!(
        keywords_match.contains("struct"),
        "Grammar keywords missing 'struct'"
    );
}

#[test]
fn test_nvim_plugin_files() {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".to_string());
    let vim_path = Path::new(&manifest_dir).join("editor/nvim/track.vim");
    let lua_path = Path::new(&manifest_dir).join("editor/nvim/track.lua");

    assert!(
        vim_path.exists(),
        "track.vim does not exist at editor/nvim/track.vim"
    );
    assert!(
        lua_path.exists(),
        "track.lua does not exist at editor/nvim/track.lua"
    );

    let lua_content = fs::read_to_string(&lua_path).expect("Failed to read track.lua");
    assert!(
        lua_content.contains("import"),
        "track.lua syntax missing 'import'"
    );
    assert!(
        lua_content.contains("icon = \"T\""),
        "track.lua devicons missing flat T icon"
    );
    assert!(
        lua_content.contains("color = \"#ffffff\""),
        "track.lua devicons missing white color"
    );

    // If luac or luajit is installed on machine, test syntax parsing of track.lua
    if let Ok(output) = Command::new("luac").arg("-p").arg(&lua_path).output() {
        assert!(
            output.status.success(),
            "Lua syntax check failed for track.lua: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
}
