use anyhow::Result;

fn main() -> Result<()> {
    let report = beaver_core::migration_bundle::command(std::env::args_os().skip(1).collect())?;
    println!("{report}");
    Ok(())
}
