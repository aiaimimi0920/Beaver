use super::*;

fn bundle(root: &Path) -> Result<Bundle> {
    for name in [DEBUG, RELEASE] {
        fs::write(root.join(name), "template")?;
    }
    Ok(Bundle {
        directory: root.to_owned(),
        debug: root.join(DEBUG),
        release: root.join(RELEASE),
        version: "4.8.dev.custom_build.38b6ddee7".into(),
    })
}

#[test]
fn matching_requires_full_custom_build_version() -> Result<()> {
    let version = "4.8.dev.custom_build.38b6ddee7";
    assert_eq!(crate::export_templates::version(version)?, "4.8.dev");
    assert!(crate::export_templates::release(version).is_err());
    require_version(version, version)?;
    for other in [
        "4.8.dev.custom_build.other",
        "4.5.1.stable.official.abc",
        "4.8.dev",
    ] {
        assert!(require_version(version, other).is_err());
    }
    Ok(())
}

#[test]
fn sibling_templates_never_fall_back_when_missing_or_invalid() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let cancelled = AtomicBool::new(false);
    assert!(discover(
        &temp.path().join("Godot_v4.5.1-stable_win64.exe"),
        &cancelled
    )?
    .is_none());
    let editor = temp.path().join("godot.windows.editor.x86_64.exe");
    assert!(discover(&editor, &cancelled)
        .unwrap_err()
        .to_string()
        .contains("缺失"));
    fs::write(temp.path().join(DEBUG), b"MZ")?;
    fs::write(temp.path().join(RELEASE), b"MZ")?;
    assert!(discover(&editor, &cancelled).is_err());
    Ok(())
}

#[test]
fn binding_only_rewrites_selected_options_and_adds_missing_options() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let bundle = bundle(temp.path())?;
    let config = "[preset.0]\nname=\"One\"\n[preset.0.options]\ncustom_template/release=\"other.exe\"\n[preset.1]\nname=\"Two\"\n[preset.1.options]\ncustom_template/debug=\"\"\ncustom_template/release=\"\"\nbinary_format/embed_pck=true\n";
    let output = bundle.preset(config, "Two", temp.path())?;
    assert!(output.contains("custom_template/release=\"other.exe\""));
    assert!(output.contains(DEBUG) && output.contains(RELEASE));
    assert!(output.contains("binary_format/embed_pck=true"));
    assert_eq!(output.matches("custom_template/release=").count(), 2);
    let missing = bundle.preset("[preset.0]\nname=\"Two\"\n", "Two", temp.path())?;
    assert!(missing.contains("[preset.0.options]"));
    assert_eq!(bundle.preset(&output, "Two", temp.path())?, output);
    Ok(())
}

#[test]
fn conflicting_templates_architectures_and_ambiguous_presets_are_rejected() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let bundle = bundle(temp.path())?;
    fs::write(temp.path().join("other.exe"), "old template")?;
    let base = "[preset.0]\nname=\"Desktop\"\n[preset.0.options]\n";
    assert!(bundle
        .preset(
            &format!("{base}custom_template/release=\"res://other.exe\"\n"),
            "Desktop",
            temp.path()
        )
        .is_err());
    assert!(bundle
        .preset(
            &format!("{base}binary_format/architecture=\"arm64\"\n"),
            "Desktop",
            temp.path()
        )
        .is_err());
    assert!(bundle
        .preset(
            &format!("{base}[preset.1]\nname=\"Desktop\"\n"),
            "Desktop",
            temp.path()
        )
        .is_err());
    assert!(bundle.preset(base, "Missing", temp.path()).is_err());
    assert_eq!(
        fs::read_to_string(temp.path().join("other.exe"))?,
        "old template"
    );
    Ok(())
}
