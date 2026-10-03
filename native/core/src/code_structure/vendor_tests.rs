use super::*;
use crate::{
    code_structure::{self, Baseline, Scope},
    files::Files,
};
use std::{fs, path::Path};

fn upstream() -> Result<BTreeMap<String, Vec<u8>>> {
    let mut archive = zip::ZipArchive::new(Cursor::new(GUT))?;
    let mut files = BTreeMap::new();
    for index in 0..archive.len() {
        let mut entry = archive.by_index(index)?;
        if !entry.is_dir() {
            let path = entry.name().replace('\\', "/");
            let mut bytes = Vec::new();
            entry.read_to_end(&mut bytes)?;
            files.insert(path, bytes);
        }
    }
    Ok(files)
}

fn install(root: &Path) -> Result<()> {
    for (path, bytes) in upstream()? {
        let destination = root.join(path);
        fs::create_dir_all(destination.parent().unwrap())?;
        fs::write(destination, bytes)?;
    }
    Ok(())
}

#[test]
fn gut_bundle_matches_the_fixed_version_license_and_every_file() -> Result<()> {
    let files = upstream()?;
    let identities = gut_hashes(GUT)?;
    assert_eq!(identities.len(), files.len());
    let metadata = std::str::from_utf8(&files["addons/gut/plugin.cfg"])?;
    assert!(metadata.contains("version=\"9.4.0\""));
    let license = std::str::from_utf8(&files["addons/gut/LICENSE.md"])?;
    assert!(license.contains("MIT"));
    for (path, bytes) in &files {
        assert!(immutable(path, bytes), "Missing pinned GUT member: {path}");
    }
    let mut changed_archive = GUT.to_vec();
    changed_archive.push(0);
    assert!(gut_hashes(&changed_archive).is_err());
    assert!(gut_hashes(b"a different GUT version/archive").is_err());
    Ok(())
}

#[test]
fn gut_identity_rejects_changed_bytes_versions_unknown_files_and_path_aliases() -> Result<()> {
    let files = upstream()?;
    let path = "addons/gut/test.gd";
    let bytes = &files[path];
    let mut modified = bytes.clone();
    modified.extend(b"\n# edited\n");
    assert!(!immutable(path, &modified));
    let crlf = std::str::from_utf8(bytes)?.replace('\n', "\r\n");
    assert!(!immutable(path, crlf.as_bytes()));
    for fake in [
        "addons/gut/custom.gd",
        "addons/gut/../test.gd",
        "addons/gut/./test.gd",
        "addons/gut//test.gd",
        "addons/gut/Test.gd",
        "addons/gut2/test.gd",
        "addons/Gut/test.gd",
        "addons\\gut\\test.gd",
        "./addons/gut/test.gd",
        "/addons/gut/test.gd",
        "res://addons/gut/test.gd",
        "vendor/addons/gut/test.gd",
        "resources/packages/npr-characters/addons/gut/test.gd",
    ] {
        assert!(!immutable(fake, bytes), "Unexpected GUT identity: {fake}");
    }
    let other_version =
        std::str::from_utf8(&files["addons/gut/plugin.cfg"])?.replace("9.4.0", "9.3.0");
    assert!(!immutable(
        "addons/gut/plugin.cfg",
        other_version.as_bytes()
    ));
    assert!(!immutable(path, &files["addons/gut/gut.gd"]));
    Ok(())
}

#[test]
fn gut_project_scan_allows_upstream_but_keeps_user_code_limits() -> Result<()> {
    let root = tempfile::tempdir()?;
    install(root.path())?;
    let report = code_structure::scan(root.path(), Scope::Game, &Baseline::empty(), true)?;
    assert!(report.ok, "{:?}", report.violations);
    assert!(report.files.iter().all(|entry| entry.status == "immutable"));
    let file = report
        .files
        .iter()
        .find(|entry| entry.path == "addons/gut/test.gd")
        .unwrap();
    assert!(file.stamp.effective_lines > 700);
    let oversized = (0..701)
        .map(|i| format!("var item_{i} = {i}\n"))
        .collect::<String>();
    for path in ["game.gd", "addons/gut/user_code.gd"] {
        fs::write(root.path().join(path), &oversized)?;
    }
    // A project-owned manifest cannot expand the compiled-in allowlist.
    fs::write(root.path().join("addons/gut/provenance.json"), "{}")?;
    let rejected = code_structure::scan(root.path(), Scope::Game, &Baseline::empty(), true)?;
    assert!(!rejected.ok);
    assert_eq!(rejected.violations.len(), 2);
    Ok(())
}

#[test]
fn gut_task_gate_checks_captured_bytes_without_creating_legacy_debt() -> Result<()> {
    let root = tempfile::tempdir()?;
    let project = root.path().join("project");
    fs::create_dir(&project)?;
    let files = Files::new(root.path().join("data"));
    let before = files.capture(&project)?;
    install(&project)?;
    let pinned = files.capture(&project)?;
    let changes = Files::changes(&before, &pinned);
    let accepted = code_structure::check_changes(&files, &before, &changes)?;
    assert!(accepted.ok, "{:?}", accepted.violations);
    assert!(code_structure::snapshot_baseline(&files, &pinned)?
        .files
        .is_empty());

    let path = "addons/gut/test.gd";
    let original = fs::read(project.join(path))?;
    let mut modified = original.clone();
    modified.extend(b"\n# changed after installation\n");
    let text = std::str::from_utf8(&original)?;
    let crlf = text.replace('\n', "\r\n").into_bytes();
    let cr = text.replace('\n', "\r").into_bytes();
    for normalized in [&crlf, &cr] {
        assert_eq!(
            code_structure::measure(path, &original)?,
            code_structure::measure(path, normalized)?
        );
    }
    for changed_bytes in [modified, crlf, cr] {
        fs::write(project.join(path), changed_bytes)?;
        let changed = files.capture(&project)?;
        let changes = Files::changes(&pinned, &changed);
        fs::write(project.join(path), &original)?;
        // A repaired live copy does not bless an already captured modified blob.
        let rejected = code_structure::check_changes(&files, &pinned, &changes)?;
        assert!(!rejected.ok);
        assert_eq!(rejected.files[0].path, path);
        assert_ne!(rejected.files[0].status, "immutable");
        assert_ne!(rejected.files[0].status, "legacy");
    }
    Ok(())
}
