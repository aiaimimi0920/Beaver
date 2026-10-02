use super::*;
#[tokio::test]
async fn download_hash_bounds_and_exclusive_destination() -> Result<()> {
    use sha2::{Digest, Sha512};
    use std::{
        io::{Read, Write},
        net::TcpListener,
    };
    for (body, expected_body, max, exists, success) in [
        ("fixture", "fixture", 32, false, true),
        ("tampered", "fixture", 32, false, false),
        ("fixture", "fixture", 3, false, false),
        ("fixture", "fixture", 32, true, false),
    ] {
        let listener = TcpListener::bind("127.0.0.1:0")?;
        let base = format!("http://{}", listener.local_addr()?);
        let sums = format!(
            "{:x}  fixture.tpz",
            Sha512::digest(expected_body.as_bytes())
        );
        let server = std::thread::spawn(move || -> std::io::Result<()> {
            for content in [sums.as_bytes(), body.as_bytes()] {
                let (mut connection, _) = listener.accept()?;
                connection.set_read_timeout(Some(std::time::Duration::from_secs(5)))?;
                let mut request = [0; 4096];
                let _ = connection.read(&mut request)?;
                write!(
                    connection,
                    "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    content.len()
                )?;
                connection.write_all(content)?;
            }
            Ok(())
        });
        let temp = tempfile::tempdir()?;
        let destination = temp.path().join("fixture.tpz");
        if exists {
            std::fs::write(&destination, b"preserved")?;
        }
        let client = reqwest::Client::builder()
            .no_proxy()
            .timeout(std::time::Duration::from_secs(5))
            .build()?;
        let release = Release {
            version: "4.6.stable".into(),
            filename: "fixture.tpz".into(),
            base,
        };
        let result = download_from(&client, &release, &destination, max).await;
        assert_eq!(result.is_ok(), success);
        if success {
            assert_eq!(std::fs::read(&destination)?, body.as_bytes());
        }
        if exists {
            assert_eq!(std::fs::read(&destination)?, b"preserved");
        }
        server.join().unwrap()?;
    }
    let temp = tempfile::tempdir()?;
    let destination = temp.path().join("cancelled.tpz");
    assert!(download(
        "4.6.stable.official.a",
        &destination,
        &std::sync::atomic::AtomicBool::new(true)
    )
    .await
    .is_err());
    assert!(!destination.exists());
    Ok(())
}
#[test]
#[cfg(windows)]
fn archive_install_is_allowlisted_versioned_and_non_overwriting() -> Result<()> {
    use base64::Engine;
    use std::sync::atomic::AtomicBool;
    let temp = tempfile::tempdir()?;
    let root = temp.path().join("template space ' & test");
    std::fs::create_dir(&root)?;
    let quote = |value: &str| format!("'{}'", value.replace('\'', "''"));
    let script = format!(
        "$Root = {}\n{}",
        quote(&crate::reveal::windows_path(&root)?),
        r#"
$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.IO.Compression
Add-Type -AssemblyName System.IO.Compression.FileSystem
foreach ($mode in @('valid', 'duplicate', 'mismatch', 'badmagic')) {
  $zip = [IO.Compression.ZipFile]::Open([IO.Path]::Combine($Root, $mode + '.tpz'), [IO.Compression.ZipArchiveMode]::Create)
  try {
    $entry = $zip.CreateEntry('templates/version.txt'); $writer = [IO.StreamWriter]::new($entry.Open())
    try { if ($mode -eq 'mismatch') { $writer.Write('4.5.stable') } else { $writer.Write('4.6.stable') } } finally { $writer.Dispose() }
    $names = @('windows_release_x86_64.exe', 'windows_debug_x86_64.exe', '../../outside.txt')
    if ($mode -eq 'duplicate') { $names += 'windows_release_x86_64.exe' }
    foreach ($name in $names) {
      $entry = $zip.CreateEntry('templates/' + $name); $stream = $entry.Open()
      try { $bytes = New-Object byte[] 1024; if ($mode -ne 'badmagic') { $bytes[0] = 77; $bytes[1] = 90 }; $stream.Write($bytes, 0, $bytes.Length) } finally { $stream.Dispose() }
    }
  } finally { $zip.Dispose() }
}
"#
    );
    let encoded = base64::engine::general_purpose::STANDARD.encode(
        script
            .encode_utf16()
            .flat_map(u16::to_le_bytes)
            .collect::<Vec<_>>(),
    );
    let executable = PathBuf::from(std::env::var_os("SystemRoot").context("SystemRoot")?)
        .join("System32/WindowsPowerShell/v1.0/powershell.exe");
    let result = crate::process::run(
        &executable,
        &["-NoProfile", "-NonInteractive", "-EncodedCommand", &encoded],
        None,
        std::time::Duration::from_secs(30),
    )?;
    anyhow::ensure!(result.code == 0, "{}", result.text);
    for mode in ["duplicate", "mismatch", "badmagic"] {
        let destination = root.join(format!("installed-{mode}"));
        assert!(install_archive(
            &root.join(format!("{mode}.tpz")),
            &destination,
            "4.6.stable",
            &AtomicBool::new(false)
        )
        .is_err());
        assert!(!destination.exists());
    }
    let destination = root.join("installed-valid");
    install_archive(
        &root.join("valid.tpz"),
        &destination,
        "4.6.stable",
        &AtomicBool::new(false),
    )?;
    assert!(ready(&destination));
    assert_eq!(std::fs::read_dir(&destination)?.count(), 3);
    assert_eq!(
        std::fs::read_to_string(destination.join("version.txt"))?,
        "4.6.stable"
    );
    assert!(!root.join("outside.txt").exists());
    assert!(!temp.path().join("outside.txt").exists());
    assert!(install_archive(
        &root.join("valid.tpz"),
        &destination,
        "4.6.stable",
        &AtomicBool::new(false)
    )
    .is_err());
    assert!(install_archive(
        &root.join("valid.tpz"),
        &root.join("cancelled"),
        "4.6.stable",
        &AtomicBool::new(true)
    )
    .is_err());
    assert!(!root.join("cancelled").exists());
    for entry in std::fs::read_dir(&root)? {
        assert!(!entry?
            .file_name()
            .to_string_lossy()
            .starts_with(".beaver-templates-"));
    }
    Ok(())
}
#[test]
fn exact_versions_and_unique_official_checksums() -> Result<()> {
    assert_eq!(version(" 4.6.stable.official.abc\n")?, "4.6.stable");
    assert_eq!(version("4.5.2.rc1.custom.abc")?, "4.5.2.rc1");
    let r = release("4.6.stable.official.abc")?;
    assert_eq!(r.version, "4.6.stable");
    assert_eq!(r.filename, "Godot_v4.6-stable_export_templates.tpz");
    assert!(r.base.ends_with("/4.6-stable"));
    for invalid in [
        "3.6.stable",
        "4.6.rc1.official.a",
        "4.6.stable.mono.official.a",
        "4.6.stable.custom.a",
    ] {
        assert!(release(invalid).is_err());
    }
    let hash = "AB".repeat(64);
    let line = format!("{hash} *{}", r.filename);
    assert_eq!(checksum(&line, &r.filename)?, hash.to_ascii_lowercase());
    assert!(checksum(&format!("{line}\n{line}"), &r.filename).is_err());
    assert!(checksum(&line, "wrong.tpz").is_err());
    assert!(checksum("deadbeef target.tpz", "target.tpz").is_err());
    Ok(())
}
#[test]
fn portable_directory_and_ready_require_both_real_executables() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let executable = temp.path().join("godot.exe");
    assert!(directory(&executable, "4.6.stable/../../elsewhere", Some(temp.path())).is_err());
    assert!(directory(&executable, "4.6.stable", Some(Path::new("relative"))).is_err());
    assert_eq!(
        directory(&executable, "4.6.stable", Some(temp.path()))?,
        temp.path().join("Godot/export_templates/4.6.stable")
    );
    std::fs::write(temp.path().join("._sc_"), b"")?;
    let target = directory(&executable, "4.6.stable", None)?;
    assert_eq!(
        target,
        temp.path().join("editor_data/export_templates/4.6.stable")
    );
    assert!(!ready(&target));
    std::fs::create_dir_all(&target)?;
    let mut bytes = vec![0; 1024];
    bytes[..2].copy_from_slice(b"MZ");
    std::fs::write(target.join(REQUIRED[0]), &bytes)?;
    assert!(!ready(&target));
    std::fs::write(target.join(REQUIRED[1]), &bytes)?;
    assert!(ready(&target));
    std::fs::write(target.join(REQUIRED[1]), b"MZ")?;
    assert!(!ready(&target));
    Ok(())
}
