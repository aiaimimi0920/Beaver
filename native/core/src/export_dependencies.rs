use crate::files::safe_path;
use anyhow::{bail, Context, Result};
use object::{endian::LittleEndian, read::ReadCache, Object};
use std::{collections::BTreeSet, fs, path::Path};

fn imports(path: &Path) -> Result<(object::Architecture, Vec<String>)> {
    // Read only PE headers/sections, not the potentially large embedded game pack.
    let cache = ReadCache::new(fs::File::open(path)?);
    let file = object::File::parse(&cache)?;
    let architecture = file.architecture();
    let table = match file {
        object::File::Pe32(file) => file.import_table()?,
        object::File::Pe64(file) => file.import_table()?,
        _ => bail!("Windows export dependency is not a PE file"),
    };
    let mut names = BTreeSet::new();
    if let Some(table) = table {
        let mut descriptors = table.descriptors()?;
        while let Some(descriptor) = descriptors.next()? {
            let name = std::str::from_utf8(table.name(descriptor.name.get(LittleEndian))?)?;
            if name.is_empty()
                || !name
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b"_.-".contains(&b))
                || !name.to_ascii_lowercase().ends_with(".dll")
            {
                bail!("Invalid native dependency name");
            }
            names.insert(name.to_ascii_lowercase());
            if names.len() > 512 {
                bail!("Too many native dependencies");
            }
        }
    }
    Ok((architecture, names.into_iter().collect()))
}

fn lookup(root: &Path, name: &str) -> Result<Option<std::path::PathBuf>> {
    let mut found = None;
    for entry in fs::read_dir(root)? {
        let entry = entry?;
        let filename = entry.file_name();
        let Some(filename) = filename.to_str() else {
            continue;
        };
        if filename.eq_ignore_ascii_case(name) {
            if found.is_some() {
                bail!("Ambiguous native dependency: {name}");
            }
            let path = safe_path(root, filename)?;
            if !path.is_file() {
                bail!("Native dependency is not a regular file: {name}");
            }
            found = Some(path);
        }
    }
    Ok(found)
}

/// Supplement only explicitly selected custom-template libraries. Never search PATH
/// or system directories. Unresolved imports are left to the Windows loader.
pub fn collect(folder: &Path, entry: &str, template_directory: &Path) -> Result<()> {
    let mut pending = vec![safe_path(folder, entry)?];
    let target_architecture = imports(&pending[0])?.0;
    let mut visited = BTreeSet::new();
    while let Some(binary) = pending.pop() {
        let (architecture, names) =
            imports(&binary).with_context(|| format!("Read imports: {}", binary.display()))?;
        if architecture != target_architecture {
            bail!(
                "Native dependency architecture mismatch: {}",
                binary.display()
            );
        }
        for name in names {
            if !visited.insert(name.clone()) {
                continue;
            }
            if visited.len() > 512 {
                bail!("Too many transitive native dependencies");
            }
            if let Some(existing) = lookup(folder, &name)? {
                pending.push(existing);
                continue;
            }
            if let Some(source) = lookup(template_directory, &name)? {
                let destination = safe_path(folder, &name)?;
                let mut input = fs::File::open(source)?;
                let mut output = fs::OpenOptions::new()
                    .write(true)
                    .create_new(true)
                    .open(&destination)?;
                std::io::copy(&mut input, &mut output)?;
                output.sync_all()?;
                pending.push(destination);
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pe(name: &str) -> Vec<u8> {
        let mut bytes = vec![0; 1024];
        bytes[..2].copy_from_slice(b"MZ");
        bytes[60..64].copy_from_slice(&128u32.to_le_bytes());
        bytes[128..132].copy_from_slice(b"PE\0\0");
        bytes[132..134].copy_from_slice(&0x8664u16.to_le_bytes());
        bytes[134..136].copy_from_slice(&1u16.to_le_bytes());
        bytes[148..150].copy_from_slice(&240u16.to_le_bytes());
        bytes[152..154].copy_from_slice(&0x20bu16.to_le_bytes());
        bytes[260..264].copy_from_slice(&16u32.to_le_bytes());
        bytes[272..276].copy_from_slice(&0x1000u32.to_le_bytes());
        bytes[276..280].copy_from_slice(&40u32.to_le_bytes());
        bytes[392..400].copy_from_slice(b".rdata\0\0");
        bytes[400..404].copy_from_slice(&512u32.to_le_bytes());
        bytes[404..408].copy_from_slice(&0x1000u32.to_le_bytes());
        bytes[408..412].copy_from_slice(&512u32.to_le_bytes());
        bytes[412..416].copy_from_slice(&512u32.to_le_bytes());
        bytes[524..528].copy_from_slice(&0x1040u32.to_le_bytes());
        bytes[576..576 + name.len()].copy_from_slice(name.as_bytes());
        bytes
    }

    #[test]
    fn only_imported_libraries_are_copied_recursively_without_overwrite() -> Result<()> {
        let root = tempfile::tempdir()?;
        let folder = root.path().join("export");
        let template = root.path().join("template");
        fs::create_dir(&folder)?;
        fs::create_dir(&template)?;
        fs::write(folder.join("Game.exe"), pe("First.DLL"))?;
        fs::write(template.join("first.dll"), pe("second.dll"))?;
        fs::write(template.join("second.dll"), pe("kernel32.dll"))?;
        fs::write(template.join("unused.dll"), b"not imported")?;
        collect(&folder, "Game.exe", &template)?;
        assert!(folder.join("first.dll").is_file());
        assert!(folder.join("second.dll").is_file());
        assert!(!folder.join("unused.dll").exists());
        assert!(!folder.join("kernel32.dll").exists());
        fs::write(template.join("first.dll"), b"do not overwrite")?;
        collect(&folder, "Game.exe", &template)?;
        assert_eq!(fs::read(folder.join("first.dll"))?, pe("second.dll"));
        Ok(())
    }

    #[test]
    fn rejects_path_imports_and_invalid_binaries() -> Result<()> {
        let root = tempfile::tempdir()?;
        let path = root.path().join("Game.exe");
        fs::write(&path, pe("../escape.dll"))?;
        assert!(imports(&path).is_err());
        fs::write(&path, b"MZ invalid")?;
        assert!(imports(&path).is_err());
        Ok(())
    }

    #[test]
    fn rejects_mismatched_dependency_architecture() -> Result<()> {
        let root = tempfile::tempdir()?;
        let template = tempfile::tempdir()?;
        fs::write(root.path().join("Game.exe"), pe("wrong.dll"))?;
        let mut wrong = pe("kernel32.dll");
        wrong[132..134].copy_from_slice(&0xaa64u16.to_le_bytes());
        fs::write(template.path().join("wrong.dll"), wrong)?;
        assert!(collect(root.path(), "Game.exe", template.path()).is_err());
        Ok(())
    }
}
