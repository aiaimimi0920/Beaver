"""Negative checks for the standalone artifact verifier; no model execution."""
from pathlib import Path
import tempfile
import unittest
import verify_capture as capture


class VerifyCaptureTests(unittest.TestCase):
    def test_valid_scale(self):
        capture.verify_scale(.4304322425, .3658674061)

    def test_consistently_wrong_scale_is_rejected(self):
        with self.assertRaisesRegex(ValueError, "Wrong camera scale"):
            capture.verify_scale(1, .95)

    def test_invalid_camera_numbers_are_rejected(self):
        for value in [0, -1, float("nan"), float("inf"), True, "1"]:
            with self.subTest(value=value):
                with self.assertRaises(ValueError):
                    capture.verify_scale(value, .85)
                with self.assertRaises(ValueError):
                    capture.verify_scale(1, value)

    def test_missing_runtime_root_is_rejected(self):
        with tempfile.TemporaryDirectory() as directory:
            with self.assertRaisesRegex(ValueError, "Missing runtime directory"):
                capture.verify_runtime(Path(directory).resolve(), {})

    def test_empty_runtime_roots_are_rejected(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory).resolve()
            for resource in capture.RUNTIME_ROOTS:
                (root / resource[6:]).mkdir(parents=True)
            with self.assertRaisesRegex(ValueError, "Missing required renderer"):
                capture.verify_runtime(root, {})

    def test_runtime_closure_requires_each_dependency(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory).resolve()
            bindings = {}
            for resource in capture.RUNTIME_SENTINELS:
                path = root / resource[6:]
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_text("fixture")
                bindings[resource] = capture.sha(path)
            capture.verify_runtime(root, bindings)
            extra = root / "showcase_extensions/extra.gd"
            extra.write_text("fixture")
            with self.assertRaisesRegex(ValueError, "Missing runtime dependency"):
                capture.verify_runtime(root, bindings)

    def test_manifest_cannot_escape_workspace(self):
        with tempfile.TemporaryDirectory() as directory:
            with self.assertRaisesRegex(ValueError, "Manifest is outside workspace"):
                capture.verify(directory, Path(directory).parent / "outside.json")

    def test_require_is_not_an_optimizable_assert(self):
        with self.assertRaises(ValueError):
            capture.require(False, "failure")


if __name__ == "__main__":
    unittest.main()
