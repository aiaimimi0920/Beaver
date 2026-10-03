//! Lightweight container checks supplement real process exit and file hashes.
//! They do not establish model quality, NPR compliance or visual acceptance.
use anyhow::{ensure, Result};
use std::{fs, io::Read, path::Path};

pub(super) fn validate(path: &Path) -> Result<()> {
    let metadata = fs::symlink_metadata(path)?;
    ensure!(
        metadata.is_file() && !crate::files::linked(&metadata),
        "Output must be a regular file"
    );
    ensure!(
        metadata.len() > 0 && metadata.len() <= super::files::ASSET_LIMIT,
        "Output is empty or exceeds 512 MiB"
    );
    let mut header = [0u8; 16];
    let length = fs::File::open(path)?.read(&mut header)?;
    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    match extension.as_str() {
        "blend" => ensure!(
            length >= 12 && &header[..7] == b"BLENDER",
            "Output is not an uncompressed Blender file"
        ),
        "glb" => {
            ensure!(length >= 12 && &header[..4] == b"glTF", "Output is not GLB");
            ensure!(
                u32::from_le_bytes(header[4..8].try_into()?) == 2
                    && u32::from_le_bytes(header[8..12].try_into()?) as u64 == metadata.len(),
                "GLB version or length is invalid"
            );
        }
        "png" => ensure!(
            length >= 8 && &header[..8] == b"\x89PNG\r\n\x1a\n",
            "Output is not PNG"
        ),
        "jpg" | "jpeg" => ensure!(
            length >= 3 && &header[..3] == b"\xff\xd8\xff",
            "Output is not JPEG"
        ),
        "exr" => ensure!(
            length >= 4 && header[..4] == [0x76, 0x2f, 0x31, 0x01],
            "Output is not OpenEXR"
        ),
        _ => {}
    }
    Ok(())
}
