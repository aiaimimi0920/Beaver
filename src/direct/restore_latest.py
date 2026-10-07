"""Restore a verified private checkpoint without restoring task state."""
import hashlib
import json
from pathlib import Path
import stat
import sys
import zipfile


def restore(archive, root, expected_root):
    """Keep original resource bytes and record their hashes at a fixed target."""
    archive, root, expected_root = map(Path, (archive, root, expected_root))
    if root.resolve() != expected_root.resolve():
        raise ValueError("Unexpected recovery destination")
    report = []
    with zipfile.ZipFile(archive) as bundle:
        selected = []
        for item in bundle.infolist():
            path = Path(item.filename)
            if path.is_absolute() or ".." in path.parts:
                raise ValueError("Unsafe archive path")
            if item.is_dir() or path.parts[0] not in ("project", "Aster-head-project"):
                continue
            if stat.S_ISLNK(item.external_attr >> 16):
                raise ValueError("Unexpected archive symlink")
            relative = Path(*path.parts[1:])
            if any(part in (".beaver", ".godot", ".git", ".aws", ".codex")
                   for part in relative.parts):
                continue
            destination = root / relative
            if not destination.resolve().is_relative_to(root.resolve()):
                raise ValueError("Destination escapes recovery root")
            selected.append((item, relative, destination))
        for item, relative, destination in selected:
            raw = bundle.read(item)
            destination.parent.mkdir(parents=True, exist_ok=True)
            destination.write_bytes(raw)
            report.append({"path": str(relative),
                           "sha256": hashlib.sha256(raw).hexdigest()})
    digest = hashlib.sha256(archive.read_bytes()).hexdigest()
    proof = {"source_archive_sha256": digest, "files": report,
             "note": "Original asset and pinned package bytes; fresh task state remains separate."}
    (root / "recovery-import.json").write_text(json.dumps(proof, indent=2), encoding="utf-8")
    return {"restored_files": len(report), "archive_sha256": digest}


if __name__ == "__main__":
    expected = Path(__file__).resolve().parents[3] / "restored-projects" / "Aster Restored"
    print(json.dumps(restore(sys.argv[1], sys.argv[2], expected)))
