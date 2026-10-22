"""Actual capture entry aliases must remain source-bound during verification."""
from pathlib import Path
import tempfile
import unittest
import verify_capture as capture


class EntrySceneTests(unittest.TestCase):
    def fixture(self, root):
        canonical = root / "showcase/faye_face_capture.tscn"
        canonical.parent.mkdir()
        canonical.write_text("capture scene fixture")
        alias = canonical.with_name("aster_head_review.tscn")
        alias.write_bytes(canonical.read_bytes())
        digest = capture.sha(canonical)
        return alias, {"entry_scene_at_capture": "res://showcase/aster_head_review.tscn",
                       "entry_scene_sha256": digest, "entry_scene_sha256_before": digest,
                       "entry_scene_sha256_after": digest,
                       "bindings_before": {"res://showcase/faye_face_capture.tscn": digest}}

    def test_actual_alias_modification_is_rejected(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory).resolve()
            alias, data = self.fixture(root)
            capture.verify_entry_scene(root, data)
            alias.write_text("restored original scene")
            with self.assertRaises(ValueError):
                capture.verify_entry_scene(root, data)
            alias.unlink()
            with self.assertRaises(ValueError):
                capture.verify_entry_scene(root, data)

    def test_entry_path_is_required_and_confined(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory).resolve()
            alias, data = self.fixture(root)
            for value in [None, "", "/tmp/scene.tscn", "res://../scene.tscn", "res://showcase"]:
                with self.subTest(value=value), self.assertRaises(ValueError):
                    capture.verify_entry_scene(root, {**data, "entry_scene_at_capture": value})
            missing = dict(data)
            del missing["entry_scene_at_capture"]
            with self.assertRaises(ValueError):
                capture.verify_entry_scene(root, missing)


if __name__ == "__main__":
    unittest.main()
