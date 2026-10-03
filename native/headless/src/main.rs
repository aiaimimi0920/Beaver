use anyhow::{bail, Context, Result};
use beaver_headless::{protocol, Host};
use std::path::PathBuf;

fn main() -> Result<()> {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    if args.len() == 1 && args[0] == beaver_core::media_server::MODE_ARGUMENT {
        return beaver_core::media_server::run_stdio();
    }
    if args.len() == 1 && (args[0] == "--help" || args[0] == "-h") {
        eprintln!("beaver-headless --data-dir ABSOLUTE_DIRECTORY\nJSONL stdio business API. Use --media-mcp only for Core media MCP.");
        return Ok(());
    }
    if args.len() != 2 || args[0] != "--data-dir" {
        bail!("Usage: beaver-headless --data-dir ABSOLUTE_DIRECTORY");
    }
    let root = PathBuf::from(&args[1]);
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()?;
    let result = runtime.block_on(run(root));
    // Tokio stdin may retain a blocking OS read after SIGTERM. All admitted work
    // and the scheduler have drained; do not wait for another stdin byte to exit.
    runtime.shutdown_background();
    result
}

async fn run(root: PathBuf) -> Result<()> {
    let (sender, receiver) = tokio::sync::watch::channel(false);
    #[cfg(unix)]
    let mut terminate = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
        .context("Could not register SIGTERM handler")?;
    let signal = tokio::spawn(async move {
        #[cfg(unix)]
        tokio::select! { _ = tokio::signal::ctrl_c() => {}, _ = terminate.recv() => {} }
        #[cfg(not(unix))]
        let _ = tokio::signal::ctrl_c().await;
        let _ = sender.send(true);
    });
    let result = match Host::open(&root) {
        Ok(host) => protocol::serve(host, tokio::io::stdin(), tokio::io::stdout(), receiver).await,
        Err(error) => Err(error),
    };
    signal.abort();
    result
}
