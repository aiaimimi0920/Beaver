use super::*;
use crate::{project_runtime::ProjectRuntime, project_storage::ProjectStore};

const PRESETS: &str = "[preset.0]\nname=\"Desktop\"\nplatform=\"Windows Desktop\"\n";

struct Fixture {
    runtime: ProjectRuntime,
    root: PathBuf,
    scratch: PathBuf,
    destination: PathBuf,
    _temp: tempfile::TempDir,
}

impl Fixture {
    fn new() -> Result<Self> {
        let temp = tempfile::tempdir()?;
        let root = temp.path().join("project");
        let scratch = temp.path().join("host");
        let destination = temp.path().join("exports");
        for path in [&root, &scratch, &destination] {
            fs::create_dir(path)?;
        }
        fs::write(root.join("project.godot"), "config_version=5\n")?;
        fs::write(root.join("export_presets.cfg"), PRESETS)?;
        fs::write(root.join("scene.tscn"), "frozen scene")?;
        let runtime = ProjectStore::initialize(&root, "p")?.into_runtime();
        Ok(Self {
            runtime,
            root,
            scratch,
            destination,
            _temp: temp,
        })
    }

    fn project(&self) -> Value {
        json!({"id":"p", "name":"Game", "path":self.root})
    }

    fn prepare(&self, snapshot: &Snapshot) -> Result<Prepared> {
        prepare_snapshot(
            &self.runtime.files(),
            &self.scratch,
            &self.project(),
            &self.destination,
            "Desktop",
            snapshot,
        )
    }
}

#[test]
fn internal_export_freezes_project_content_in_separate_scratch() -> Result<()> {
    let fixture = Fixture::new()?;
    fs::create_dir(fixture.root.join("templates"))?;
    fs::write(fixture.root.join("templates/release.exe"), "template")?;
    fs::write(
        fixture.root.join("export_presets.cfg"),
        format!(
        "{PRESETS}[preset.0.options]\ncustom_template/release=\"res://templates/release.exe\"\n"
    ),
    )?;
    let files = fixture.runtime.files();
    let job = prepare(
        &files,
        &fixture.scratch,
        &fixture.project(),
        &fixture.destination,
        "Desktop",
    )?;
    fs::write(fixture.root.join("scene.tscn"), "new live scene")?;
    let workspace = job.workspace.path().to_owned();
    assert_eq!(
        fs::canonicalize(workspace.parent().unwrap())?,
        fs::canonicalize(&fixture.scratch)?
    );
    assert_eq!(
        fs::read_to_string(workspace.join("scene.tscn"))?,
        "frozen scene"
    );
    assert_eq!(
        job.template_directory,
        Some(fs::canonicalize(workspace.join("templates"))?)
    );
    assert!(!workspace.join(".beaver").exists());
    assert!(files
        .blob(&files.capture(&fixture.root)?["project.godot"])?
        .is_file());
    assert!(!fixture.scratch.join("blobs").exists());
    assert!(fs::read_dir(&fixture.destination)?.next().is_none());
    drop(job);
    assert!(!workspace.exists());
    Ok(())
}

#[test]
fn frozen_export_survives_relocation_and_ignores_live_presets() -> Result<()> {
    let fixture = Fixture::new()?;
    let snapshot = fixture.runtime.files().capture(&fixture.root)?;
    let Fixture {
        runtime,
        root,
        scratch,
        destination,
        _temp,
    } = fixture;
    drop(runtime);
    let moved = _temp.path().join("moved");
    fs::rename(root, &moved)?;
    let runtime = ProjectStore::open(&moved, "p")?.into_runtime();
    fs::write(moved.join("export_presets.cfg"), "invalid live preset")?;
    fs::write(moved.join("scene.tscn"), "new live scene")?;
    snapshot_preset(&runtime.files(), &snapshot, "Desktop")?;
    let job = prepare_snapshot(
        &runtime.files(),
        &scratch,
        &json!({"path":moved}),
        &destination,
        "Desktop",
        &snapshot,
    )?;
    drop(runtime);
    assert_eq!(job.target.name, "Desktop");
    assert_eq!(
        fs::read_to_string(job.workspace.path().join("scene.tscn"))?,
        "frozen scene"
    );
    assert_eq!(
        fs::read_to_string(job.workspace.path().join("export_presets.cfg"))?,
        PRESETS
    );
    assert!(!scratch.join("blobs").exists());
    Ok(())
}

#[test]
fn missing_or_corrupt_project_blobs_never_fall_back_to_host_content() -> Result<()> {
    let fixture = Fixture::new()?;
    let files = fixture.runtime.files();
    let snapshot = files.capture(&fixture.root)?;
    let legacy = Files::new(fixture.scratch.clone());
    assert_eq!(legacy.capture(&fixture.root)?, snapshot);
    for relative in ["scene.tscn", "export_presets.cfg"] {
        let blob = files.blob(&snapshot[relative])?;
        let original = fs::read(&blob)?;
        fs::remove_file(&blob)?;
        assert!(fixture.prepare(&snapshot).is_err());
        fs::write(&blob, "corrupt")?;
        assert!(fixture.prepare(&snapshot).is_err());
        fs::write(blob, original)?;
    }
    assert_eq!(fs::read_dir(&fixture.scratch)?.count(), 1);
    let job = fixture.prepare(&snapshot)?;
    assert_eq!(
        fs::read_to_string(job.workspace.path().join("scene.tscn"))?,
        "frozen scene"
    );
    Ok(())
}

#[test]
fn legacy_export_keeps_external_templates_and_destination_rules() -> Result<()> {
    let fixture = Fixture::new()?;
    let template = fixture._temp.path().join("release.exe");
    fs::write(&template, "external template")?;
    fs::write(
        fixture.root.join("export_presets.cfg"),
        format!(
            "{PRESETS}[preset.0.options]\ncustom_template/release={}\n",
            serde_json::to_string(&template)?
        ),
    )?;
    let files = Files::new(fixture.scratch.clone());
    let snapshot = files.capture(&fixture.root)?;
    let job = prepare(
        &files,
        &fixture.scratch,
        &fixture.project(),
        &fixture.destination,
        "Desktop",
    )?;
    assert_eq!(
        job.template_directory,
        Some(fs::canonicalize(fixture._temp.path())?)
    );
    assert!(!job.workspace.path().join("release.exe").exists());
    assert!(prepare_snapshot(
        &files,
        &fixture.scratch,
        &fixture.project(),
        &fixture.root,
        "Desktop",
        &snapshot
    )
    .is_err());
    assert_eq!(files.capture(&fixture.root)?, snapshot);
    assert_eq!(fs::read_to_string(template)?, "external template");
    Ok(())
}
