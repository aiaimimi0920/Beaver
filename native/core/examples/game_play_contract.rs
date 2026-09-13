use anyhow::{ensure, Context, Result};
use beaver_core::game_play::Players;
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
    time::Duration,
};

async fn wait_for(path: &Path) -> Result<()> {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    while !path.exists() {
        ensure!(
            tokio::time::Instant::now() < deadline,
            "fixture marker not created: {}",
            path.display()
        );
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    let args: Vec<_> = std::env::args_os().collect();
    if args.get(1).and_then(|arg| arg.to_str()) == Some("--descendant") {
        let root = PathBuf::from(args.get(2).context("project required")?);
        fs::write(root.join("started"), std::process::id().to_string())?;
        std::thread::sleep(Duration::from_secs(4));
        fs::write(root.join("survived"), "descendant survived cleanup")?;
        return Ok(());
    }
    if args.get(1).and_then(|arg| arg.to_str()) == Some("--path") {
        let root = PathBuf::from(args.get(2).context("project required")?);
        let mode = fs::read_to_string(root.join("mode"))?;
        if mode == "early" {
            eprintln!("SCRIPT ERROR: fixture startup failure");
            return Ok(());
        }
        let mut command = std::process::Command::new(std::env::current_exe()?);
        command.arg("--descendant").arg(&root);
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            command.creation_flags(0x08000000);
        }
        let mut child = command.spawn()?;
        std::io::stdout().write_all(&vec![b'x'; 9 * 1024 * 1024])?;
        fs::write(root.join("output-drained"), "ok")?;
        if mode == "parent-exits" {
            wait_for(&root.join("started")).await?;
            std::thread::sleep(Duration::from_millis(1500));
            return Ok(());
        }
        std::thread::sleep(Duration::from_secs(20));
        child.wait()?;
        return Ok(());
    }
    let proof = PathBuf::from(args.get(1).context("proof path required")?);
    let temp = tempfile::tempdir()?;
    let exe = std::env::current_exe()?;
    let early = temp.path().join("early");
    fs::create_dir(&early)?;
    fs::write(early.join("mode"), "early")?;
    let players = Players::default();
    let error = players
        .play(&exe, &early)
        .await
        .expect_err("early exit accepted")
        .to_string();
    ensure!(error.contains("SCRIPT ERROR"), "startup diagnostic lost");
    let mut roots = Vec::new();
    for name in ["one", "two"] {
        let root = temp.path().join(name);
        fs::create_dir(&root)?;
        fs::write(root.join("mode"), "running")?;
        players.play(&exe, &root).await?;
        wait_for(&root.join("started")).await?;
        wait_for(&root.join("output-drained")).await?;
        ensure!(root.join("started").is_file(), "child never started");
        ensure!(
            root.join("output-drained").is_file(),
            "large output blocked the game"
        );
        roots.push(root);
    }
    players.shutdown().await?;
    ensure!(
        players.play(&exe, &early).await.is_err(),
        "play after shutdown accepted"
    );
    tokio::time::sleep(Duration::from_secs(5)).await;
    for root in &roots {
        ensure!(
            !root.join("survived").exists(),
            "game child survived shutdown"
        );
    }
    let starting = Players::default();
    let root = temp.path().join("starting");
    fs::create_dir(&root)?;
    fs::write(root.join("mode"), "running")?;
    let caller = starting.clone();
    let fixture = root.clone();
    let play = tokio::spawn(async move { caller.play(&exe, &fixture).await });
    let deadline = tokio::time::Instant::now() + Duration::from_secs(3);
    while !root.join("started").exists() {
        ensure!(
            tokio::time::Instant::now() < deadline,
            "startup race child never started"
        );
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    starting.shutdown().await?;
    ensure!(
        play.await?.is_err(),
        "startup shutdown returned successful play"
    );
    tokio::time::sleep(Duration::from_secs(5)).await;
    ensure!(
        !root.join("survived").exists(),
        "startup race leaked a child"
    );
    let natural = Players::default();
    let root = temp.path().join("natural");
    fs::create_dir(&root)?;
    fs::write(root.join("mode"), "parent-exits")?;
    natural.play(&std::env::current_exe()?, &root).await?;
    wait_for(&root.join("started")).await?;
    ensure!(
        root.join("started").exists(),
        "natural-exit descendant never started"
    );
    tokio::time::sleep(Duration::from_secs(5)).await;
    ensure!(
        !root.join("survived").exists(),
        "descendant outlived naturally-exited game"
    );
    natural.shutdown().await?;
    let mut checks = vec![
        "early exit includes startup errors",
        "large output drains without blocking",
        "multiple games start independently",
        "shutdown kills known started descendants",
        "play after shutdown rejected",
        "shutdown during startup rejects play and cleans child",
        "natural parent exit also cleans its descendants",
    ];
    let mut actual = serde_json::Value::Null;
    if args.len() > 2 {
        let godot = fs::canonicalize(Path::new(&args[2]))?;
        let project = fs::canonicalize(Path::new(args.get(3).context("actual project required")?))?;
        let game = Players::default();
        let original = fs::read(project.join("project.godot"))?;
        game.play(&godot, &project).await?;
        game.shutdown().await?;
        ensure!(
            fs::read(project.join("project.godot"))? == original,
            "play changed project definition"
        );
        actual = serde_json::json!({"godot":godot,"project":project});
        checks.push("actual Godot process survives startup and managed shutdown completes");
    }
    fs::create_dir_all(proof.parent().context("proof parent required")?)?;
    fs::write(
        &proof,
        serde_json::to_vec_pretty(
            &serde_json::json!({"passed":true,"checks":checks,"actual":actual}),
        )?,
    )?;
    println!("{}", proof.display());
    Ok(())
}
