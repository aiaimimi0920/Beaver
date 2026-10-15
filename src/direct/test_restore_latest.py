"""Focused tests for checkpoint fidelity and recovery boundaries."""
import hashlib
import json
from pathlib import Path
import stat
import tempfile
import unittest
import zipfile

from restore_latest import restore


class RestoreCheckpointTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.base = Path(self.temp.name)
        self.archive = self.base / "checkpoint.zip"
        self.root = self.base / "project"
        self.root.mkdir()

    def bundle(self, entries):
        with zipfile.ZipFile(self.archive, "w") as bundle:
            for name, value in entries:
                bundle.writestr(name, value)

    def test_preserves_latest_assets_and_pinned_package(self):
        self.bundle([
            ("project/old.txt", b"baseline"),
            ("Aster-head-project/assets/model.glb", b"new\x00bytes"),
            ("Aster-head-project/addons/npr/.ci_script/check.gd", b"upstream"),
            ("Aster-head-project/beaver.runtime.json", b"{\"schemaVersion\":1}"),
        ])
        result = restore(self.archive, self.root, self.root)
        self.assertEqual(result["restored_files"], 4)
        self.assertEqual((self.root / "assets/model.glb").read_bytes(), b"new\x00bytes")
        self.assertEqual((self.root / "addons/npr/.ci_script/check.gd").read_bytes(), b"upstream")
        proof = json.loads((self.root / "recovery-import.json").read_text())
        self.assertEqual(proof["source_archive_sha256"], hashlib.sha256(self.archive.read_bytes()).hexdigest())
        for item in proof["files"]:
            self.assertEqual(item["sha256"], hashlib.sha256((self.root / item["path"]).read_bytes()).hexdigest())

    def test_does_not_restore_task_state_or_caches(self):
        self.bundle([(f"Aster-head-project/{part}/state", b"old")
                     for part in (".beaver", ".godot", ".git", ".aws", ".codex")]
                    + [("unrelated/secret", b"excluded")])
        restore(self.archive, self.root, self.root)
        self.assertEqual(sorted(p.name for p in self.root.iterdir()), ["recovery-import.json"])

    def test_rejects_traversal_before_writing(self):
        self.bundle([("Aster-head-project/good.txt", b"good"),
                     ("Aster-head-project/../../outside", b"bad")])
        with self.assertRaisesRegex(ValueError, "Unsafe archive"):
            restore(self.archive, self.root, self.root)
        self.assertFalse((self.root / "good.txt").exists())

    def test_rejects_archive_symlink(self):
        item = zipfile.ZipInfo("Aster-head-project/link")
        item.external_attr = (stat.S_IFLNK | 0o777) << 16
        self.bundle([(item, b"outside")])
        with self.assertRaisesRegex(ValueError, "symlink"):
            restore(self.archive, self.root, self.root)

    def test_rejects_existing_destination_symlink(self):
        outside = self.base / "outside"
        outside.mkdir()
        (self.root / "escape").symlink_to(outside, target_is_directory=True)
        self.bundle([("Aster-head-project/escape/file", b"bad")])
        with self.assertRaisesRegex(ValueError, "escapes"):
            restore(self.archive, self.root, self.root)
        self.assertFalse((outside / "file").exists())

    def test_rejects_wrong_destination(self):
        self.bundle([("Aster-head-project/file", b"bad")])
        with self.assertRaisesRegex(ValueError, "Unexpected recovery"):
            restore(self.archive, self.root, self.base / "expected")


if __name__ == "__main__":
    unittest.main()
