use anyhow::{Context, Result};
use beaver_core::{export_templates, process};
use serde_json::json;
use std::{path::PathBuf, sync::atomic::AtomicBool, time::Duration};

#[tokio::main]
async fn main() -> Result<()> {
    let root = PathBuf::from(
        std::env::args()
            .nth(1)
            .context("fresh output directory required")?,
    );
    std::fs::create_dir(&root)?;
    let archive = root.join("templates.tpz");
    let cancelled = AtomicBool::new(false);
    let release = export_templates::release("4.6.stable.official.contract")?;
    export_templates::download("4.6.stable.official.contract", &archive, &cancelled).await?;
    let directory = root.join(&release.version);
    export_templates::install_archive(&archive, &directory, &release.version, &cancelled)?;
    let mut programs = Vec::new();
    for name in export_templates::REQUIRED {
        let executable = directory.join(name);
        let probe = process::run(
            &executable,
            &["--headless", "--version"],
            None,
            Duration::from_secs(20),
        )?;
        anyhow::ensure!(probe.code == 0, "template version probe failed");
        anyhow::ensure!(
            export_templates::version(&probe.text)? == release.version,
            "template version mismatch"
        );
        programs.push(json!({"name":name,"version":probe.text.trim(),"bytes":std::fs::metadata(executable)?.len()}));
    }
    let proof = json!({"checks":["official HTTPS template archive downloaded and SHA-512 verified",
        "only approved Windows templates installed into fresh owned directory",
        "both real installed template executables run and report matching engine version"],
        "source":format!("{}/{}", release.base, release.filename),"archiveBytes":std::fs::metadata(&archive)?.len(),
        "programs":programs,"desktopIpcVerified":false,"gameExportVerified":false});
    std::fs::write(root.join("proof.json"), serde_json::to_vec_pretty(&proof)?)?;
    println!("{}", root.join("proof.json").display());
    Ok(())
}
