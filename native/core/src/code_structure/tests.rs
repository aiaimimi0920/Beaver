use super::*;
use anyhow::Result;
use std::{collections::BTreeMap, fs};

fn lines(path: &str, source: &str) -> usize {
    measure(path, source.as_bytes()).unwrap().effective_lines
}

fn source(count: usize) -> Vec<u8> {
    (0..count)
        .map(|i| format!("var item_{i} = {i}\n"))
        .collect::<String>()
        .into_bytes()
}

#[test]
fn language_comments_do_not_hide_literals_or_following_code() {
    assert_eq!(
        lines(
            "a.gd",
            "# comment\n\nvar url = \"https://host/#item\" # inline\n"
        ),
        1
    );
    assert_eq!(
        lines("a.gd", "var text = \"\"\"\n# stored data\nvalue\n\"\"\"\n"),
        4
    );
    assert_eq!(
        lines(
            "a.rs",
            "/* outer\n/* nested */\n*/\nfn f<'a>(v: &'a str) {} // code\n// tail\n"
        ),
        1
    );
    assert_eq!(
        lines(
            "a.rs",
            "let value = br##\"\n/* data */\n\"##;\n// comment\nlet c = '\\'';\n"
        ),
        4
    );
    assert_eq!(
        lines(
            "a.ts",
            "/* comment\n * comment\n */ const x = `\n// data\n`;\n"
        ),
        3
    );
    assert_eq!(
        lines(
            "a.css",
            "/* comment */\na { background: url('https://host/x'); }\n"
        ),
        1
    );
    assert_eq!(
        lines(
            "a.gdshader",
            "// comment\n#include \"common.gdshaderinc\"\n/* block\n*/ void fragment() {}\n"
        ),
        2
    );
    assert_eq!(
        lines(
            "a.ps1",
            "<# comment\n#>\n$x = @'\n# data\n'@\n# comment\nWrite-Output $x\n"
        ),
        4
    );
    assert_eq!(
        lines(
            "a.ps1",
            "$x = 'can''t # comment' # tail\n$x = \"a`\"# value\"\n"
        ),
        2
    );
    assert_eq!(
        lines(
            "a.html",
            "<!-- comment\n--><div title=\"<!--data-->\">x</div>\n"
        ),
        1
    );
    assert_eq!(lines("a.cmd", "@REM comment\n:: comment\n@echo value\n"), 1);
}

#[test]
fn python_docstrings_differ_from_assigned_multiline_strings() {
    let text = "\"\"\"Module docs\nMore docs\n\"\"\"\n# comment\ndef f():\n    \"\"\"Function docs\n    more\n    \"\"\"\n    return 1\nvalue = \"\"\"\n# data\n\"\"\"\n";
    assert_eq!(lines("a.py", text), 5);
}

#[test]
fn canonical_hash_preserves_content_but_normalizes_line_endings() -> Result<()> {
    let a = measure("a.gd", b"var a = 1\n# note\n")?;
    assert_eq!(a, measure("a.gd", b"var a = 1\r\n# note\r\n")?);
    assert_eq!(a, measure("a.gd", b"var a = 1\r# note\r")?);
    assert_ne!(a, measure("a.gd", b"var a = 2\n# note\n")?);
    assert!(measure("a.gd", &[0xff, 0xfe]).is_err());
    assert!(!is_source("scene.tscn"));
    assert!(is_source("SCRIPT.GD"));
    Ok(())
}

fn evaluate(count: usize, baseline: &Baseline, exceptions: &Exceptions, strict: bool) -> Report {
    check(
        BTreeMap::from([(
            "game.gd".into(),
            (measure("game.gd", &source(count)).unwrap(), false),
        )]),
        baseline,
        exceptions,
        strict,
    )
}

#[test]
fn thresholds_legacy_ratchet_and_source_identity_are_enforced() {
    for (count, status, ok) in [
        (150, "ok", true),
        (250, "ok", true),
        (251, "cohesion", true),
        (500, "cohesion", true),
        (501, "exceptionRequired", false),
        (700, "exceptionRequired", false),
        (701, "splitRequired", false),
        (1500, "splitRequired", false),
        (1501, "hardLimit", false),
    ] {
        let report = evaluate(count, &Baseline::empty(), &Exceptions::default(), false);
        assert_eq!((report.files[0].status, report.ok), (status, ok));
    }
    let stamp = measure("game.gd", &source(800)).unwrap();
    let mut baseline = Baseline::empty();
    baseline.files.insert("game.gd".into(), stamp);
    assert!(evaluate(800, &baseline, &Exceptions::default(), false).ok);
    assert!(!evaluate(800, &baseline, &Exceptions::default(), true).ok);
    baseline.files.get_mut("game.gd").unwrap().sha256 = "different content, same count".into();
    assert!(!evaluate(800, &baseline, &Exceptions::default(), false).ok);
    // Renaming a legacy oversized file cannot carry its waiver to a new owner.
    baseline.files = BTreeMap::from([("old.gd".into(), measure("old.gd", &source(800)).unwrap())]);
    assert!(!evaluate(800, &baseline, &Exceptions::default(), false).ok);
}

#[test]
fn exceptions_require_current_identity_and_protective_evidence() {
    let stamp = measure("game.gd", &source(600)).unwrap();
    let mut exceptions = Exceptions::default();
    let exception = Exception {
        effective_lines: stamp.effective_lines,
        sha256: stamp.sha256,
        responsibility: "Single parser state machine".into(),
        reason: "Keep token transitions auditable in one place".into(),
        tests: vec!["run parser contract suite".into()],
    };
    exceptions.files.insert("game.gd".into(), exception);
    assert!(evaluate(600, &Baseline::empty(), &exceptions, true).ok);
    assert!(!evaluate(601, &Baseline::empty(), &exceptions, false).ok);
    let updated = measure("game.gd", &source(701)).unwrap();
    let entry = exceptions.files.get_mut("game.gd").unwrap();
    entry.effective_lines = 701;
    entry.sha256 = updated.sha256;
    assert!(!evaluate(701, &Baseline::empty(), &exceptions, false).ok);
    exceptions.files.get_mut("game.gd").unwrap().tests.clear();
    assert!(!evaluate(600, &Baseline::empty(), &exceptions, false).ok);
    assert!(!check(BTreeMap::new(), &Baseline::empty(), &exceptions, false).ok);
}

#[test]
fn scan_excludes_build_output_but_not_user_addons() -> Result<()> {
    let root = tempfile::tempdir()?;
    for directory in ["src", "output", "addons", "node_modules"] {
        fs::create_dir(root.path().join(directory))?;
        fs::write(root.path().join(directory).join("game.gd"), source(701))?;
    }
    let report = scan(root.path(), Scope::Repository, &Baseline::empty(), false)?;
    assert_eq!(report.files.len(), 2);
    assert_eq!(report.violations.len(), 2);
    let game = scan(root.path(), Scope::Game, &Baseline::empty(), false)?;
    assert_eq!(game.files.len(), 3);
    assert_eq!(game.violations.len(), 3);
    fs::write(root.path().join(EXCEPTIONS_FILE), "{invalid")?;
    assert!(scan(root.path(), Scope::Game, &Baseline::empty(), false).is_err());
    Ok(())
}

#[test]
fn bundled_dependency_exemption_is_pinned_to_path_and_bytes() {
    let bytes = include_bytes!("../../../../resources/packages/npr-characters/addons/npr_characters/runtime/npr_multiview_transport.gd");
    let path = "addons/npr_characters/runtime/npr_multiview_transport.gd";
    assert!(vendor::immutable(path, bytes));
    assert!(vendor::immutable(
        &format!("resources/packages/npr-characters/{path}"),
        bytes
    ));
    assert!(!vendor::immutable("addons/custom.gd", bytes));
    let mut modified = bytes.to_vec();
    modified.extend(b"\n# edited\n");
    assert!(!vendor::immutable(path, &modified));
}
