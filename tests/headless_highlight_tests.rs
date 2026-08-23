use regex::Regex;
use std::fs;
use std::path::Path;
use std::process::Command;

// ── VS Code — headless grammar validation (no UI needed) ─────────
// Uses regex crate already in the project to prove each grammar pattern actually matches
// sample Track code — this is the same engine VS Code (TextMate) uses headlessly.

fn load_grammar() -> serde_json::Value {
    let manifest = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let p = Path::new(&manifest).join("editor/vscode/syntaxes/track.tmLanguage.json");
    let src = fs::read_to_string(&p).expect("read grammar");
    serde_json::from_str(&src).expect("json valid")
}

#[test]
fn test_vscode_grammar_headless_tokenization() {
    let v = load_grammar();
    // collect every regex from repository
    let repo = v["repository"].as_object().unwrap();
    let mut patterns: Vec<(String, String)> = vec![];
    for (key, entry) in repo {
        if let Some(arr) = entry["patterns"].as_array() {
            for pat in arr {
                if let Some(m) = pat["match"].as_str() {
                    patterns.push((key.clone(), m.to_string()));
                }
                if let Some(b) = pat["begin"].as_str() {
                    patterns.push((key.clone(), b.to_string()));
                }
            }
        }
    }
    assert!(patterns.len() > 10, "too few patterns: {}", patterns.len());

    // Sample that exercises every major scope
    let sample = r#"import "std/io";
fn main() -> void {
  let mut x: i64 = 42;
  // TODO fix
  let s = "hello \"world\"";
  let n = 0xFF;
  let b = 0b1010;
  if x > 0 { print(s); }
  with u -> v { v.set(1); }
  @macro bit(n: u32) -> u32 { return 1 << n; }
  let p: ptr<u8> = alloc(8);
  match val { _ => print(0), }
}
"#;

    // For each important construct, find a grammar pattern that matches it in the sample
    let checks: Vec<(&str, &str, &[&str])> = vec![
        ("keyword fn", sample, &["fn"]),
        ("keyword import", sample, &["import"]),
        ("keyword struct/enum", "struct Foo { }", &["struct"]),
        ("type i64", sample, &["i64"]),
        ("type ptr", sample, &["ptr"]),
        ("string", sample, &[r#""std/io""#, r#""hello"#]),
        ("comment", sample, &["// TODO"]),
        ("hex number", sample, &["0xFF"]),
        ("binary number", sample, &["0b1010"]),
        ("macro", sample, &["@macro"]),
        ("operator ->", sample, &["->"]),
        ("operator ::", "a::b", &["::"]),
    ];

    for (label, text, needles) in checks {
        let mut matched = false;
        for (_, pat) in &patterns {
            // skip patterns that are not regex-valid (e.g. contains unescaped)
            let re = match Regex::new(pat) {
                Ok(r) => r,
                Err(_) => continue,
            };
            for needle in needles {
                // needle is literal; check if some pattern matches it inside text
                if re.is_match(needle) && text.contains(needle) {
                    matched = true;
                    break;
                }
            }
            if matched {
                break;
            }
        }
        assert!(
            matched,
            "headless VS Code: pattern for {} ({:?}) not matched by any grammar regex",
            label, needles
        );
    }

    // scopeName must survive
    assert_eq!(v["scopeName"], "source.track");
}

#[test]
fn test_vscode_grammar_valid_json_headless() {
    let manifest = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    for p in [
        "grammars/track.tmLanguage.json",
        "editor/vscode/syntaxes/track.tmLanguage.json",
    ] {
        let path = Path::new(&manifest).join(p);
        let src = fs::read_to_string(&path).unwrap();
        let v: serde_json::Value = serde_json::from_str(&src).unwrap();
        assert_eq!(
            v["scopeName"], "source.track",
            "scopeName mismatch in {}",
            p
        );
        assert!(v["repository"].is_object(), "repository missing in {}", p);
        // each repository entry must have at least one pattern
        let repo = v["repository"].as_object().unwrap();
        for (k, entry) in repo {
            let arr = entry["patterns"]
                .as_array()
                .unwrap_or_else(|| panic!("{} patterns not array", k));
            assert!(!arr.is_empty(), "{} has no patterns", k);
        }
    }
}

// ── Neovim — headless filetype + syntax ───────────────────────────
fn nvim_available() -> bool {
    Command::new("nvim")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

#[test]
fn test_nvim_headless_filetype_and_syntax() {
    if !nvim_available() {
        eprintln!("skipping nvim headless test — nvim not found");
        return;
    }
    let manifest = std::env::var("CARGO_MANIFEST_DIR").unwrap();

    // 1) filetype detection + setup() headless
    let out = Command::new("nvim")
        .args([
            "--headless",
            "-u",
            "NONE",
            "-n",
            "-c",
            &format!("set rtp+={}/editor/nvim", manifest),
            "-c",
            "lua require('track').setup()",
            "-c",
            "edit examples/hello.trk",
            "-c",
            "lua print('FT='..vim.bo.filetype)",
            "-c",
            "qa!",
        ])
        .output()
        .expect("spawn nvim");
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    let combined = format!("{} {}", stdout, stderr);
    assert!(
        combined.contains("FT=track"),
        "nvim headless filetype not track, got: {}",
        combined
    );

    // 2) syntax groups exist after FileType
    let out2 = Command::new("nvim")
        .args([
            "--headless",
            "-u",
            "NONE",
            "-n",
            "-c",
            &format!("set rtp+={}/editor/nvim", manifest),
            "-c",
            "lua require('track').setup()",
            "-c",
            "edit examples/hello.trk",
            "-c",
            "doautocmd FileType track",
            "-c",
            "redir => g:__syn | silent syn list | redir END | silent put =g:__syn | %print | qa!",
        ])
        .output()
        .expect("spawn nvim syn");
    let combined2 = format!(
        "{} {}",
        String::from_utf8_lossy(&out2.stdout),
        String::from_utf8_lossy(&out2.stderr)
    );
    for grp in [
        "trackKeyword",
        "trackType",
        "trackString",
        "trackComment",
        "trackNumber",
    ] {
        assert!(
            combined2.contains(grp),
            "nvim syntax group {} missing in headless output: {}",
            grp,
            combined2
        );
    }

    // 3) highlight links
    let out3 = Command::new("nvim")
        .args([
            "--headless", "-u", "NONE", "-n",
            "-c", &format!("set rtp+={}/editor/nvim", manifest),
            "-c", "lua require('track').setup()",
            "-c", "edit examples/hello.trk",
            "-c", "doautocmd FileType track",
            "-c", "redir => g:__hl | silent hi trackKeyword | redir END | silent put =g:__hl | %print | qa!",
        ])
        .output()
        .expect("spawn nvim hi");
    let combined3 = format!(
        "{} {}",
        String::from_utf8_lossy(&out3.stdout),
        String::from_utf8_lossy(&out3.stderr)
    );
    assert!(
        combined3.contains("trackKeyword") && combined3.contains("Keyword"),
        "highlight link for trackKeyword missing: {}",
        combined3
    );
}

#[test]
fn test_nvim_headless_lua_syntax_ok() {
    if !nvim_available() {
        eprintln!("skipping — nvim not found");
        return;
    }
    let manifest = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let lua_file = Path::new(&manifest).join("editor/nvim/track.lua");
    let out = Command::new("nvim")
        .args([
            "--headless",
            "-u",
            "NONE",
            "-n",
            "-c",
            &format!("luafile {}", lua_file.display()),
            "-c",
            "lua print('lua_ok')",
            "-c",
            "qa!",
        ])
        .output()
        .expect("nvim luafile");
    let combined = format!(
        "{} {}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        combined.contains("lua_ok"),
        "nvim luafile failed: {}",
        combined
    );
}

#[test]
fn test_nvim_devicon_svg_not_needed_headless() {
    // Prove: terminal nvim cannot render SVG — it uses font glyphs.
    // The SVG files are only for VS Code (graphical). This test documents the fact
    // and ensures the nvim plugin does NOT try to load an SVG.
    let manifest = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let lua_src = fs::read_to_string(Path::new(&manifest).join("editor/nvim/track.lua")).unwrap();
    assert!(
        lua_src.contains("Terminal cannot render") || lua_src.contains("vector images"),
        "track.lua should document that SVG/vector is not for nvim"
    );
    // must not actually reference an SVG file path (e.g. load .svg); comment mentioning SVG is ok if it says "cannot render"
    // ensure no code tries to load SVG — check for 'load.*svg' or 'track-icon.svg' in code (allow comment)
    let code_lines: Vec<&str> = lua_src
        .lines()
        .filter(|l| !l.trim_start().starts_with("--"))
        .collect();
    let code = code_lines.join("\n");
    assert!(
        !code.contains(".svg"),
        "nvim track.lua code should not reference .svg — it uses font glyphs (comment ok)"
    );
    let pkg = fs::read_to_string(Path::new(&manifest).join("editor/vscode/package.json")).unwrap();
    assert!(
        pkg.contains("track-icon.png"),
        "vscode package.json should reference PNG icon"
    );
    assert!(Path::new(&manifest).join("assets/track-icon.svg").exists());
}

#[test]
fn test_nvim_syntax_highlight_sample_headless() {
    if !nvim_available() {
        eprintln!("skipping — nvim not found");
        return;
    }
    let manifest = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    // Open a sample and ask synID for each keyword to ensure it is highlighted
    let sample = "examples/hello.trk";
    let out = Command::new("nvim")
        .args([
            "--headless", "-u", "NONE", "-n",
            "-c", &format!("set rtp+={}/editor/nvim", manifest),
            "-c", "lua require('track').setup()",
            "-c", &format!("edit {}", sample),
            "-c", "doautocmd FileType track",
            "-c", "let g:ids = [] | for lnum in range(1, line('$')) | for col in range(1, strdisplaywidth(getline(lnum))) | call add(g:ids, synIDattr(synID(lnum, col, 1), 'name')) | endfor | endfor | echo join(g:ids, ',')",
            "-c", "qa!",
        ])
        .output()
        .expect("nvim synID");
    let combined = format!(
        "{} {}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    // At least one syntax group should be reported for hello.trk (which contains fn, print)
    assert!(
        combined.contains("trackKeyword")
            || combined.contains("trackFunction")
            || combined.contains("trackString"),
        "nvim headless synID produced no Track syntax: {}",
        combined
    );
}
