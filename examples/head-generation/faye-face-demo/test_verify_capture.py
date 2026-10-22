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


    def test_exact_neutral_channels(self):
        mouth = dict.fromkeys(capture.MOUTH_CHANNELS, 0.0)
        capture.verify_neutral(mouth, [0, 0.0])
        for bad in [{}, {**mouth, "extra": 0}, {key: value for key, value in mouth.items() if key != "MouthOpen"}]:
            with self.assertRaises(ValueError):
                capture.verify_neutral(bad, [0, 0])

    def test_neutral_values_must_be_finite_numbers(self):
        for value in [True, "0", None, float("nan"), float("inf"), .1]:
            mouth = dict.fromkeys(capture.MOUTH_CHANNELS, 0)
            mouth["MouthOpen"] = value
            with self.assertRaises(ValueError):
                capture.verify_neutral(mouth, [0, 0])
            with self.assertRaises(ValueError):
                capture.verify_neutral(dict.fromkeys(capture.MOUTH_CHANNELS, 0), [value, 0])

    def test_vectors_must_have_three_finite_numbers(self):
        capture.verify_vector([0, 1.0, -2])
        for value in [[], [0, 0], [0, 0, 0, 0], "000", [False, 0, 0], [float("nan"), 0, 0], [float("inf"), 0, 0]]:
            with self.assertRaises(ValueError):
                capture.verify_vector(value)

    def test_texture_snapshot_is_complete_and_source_bound(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory).resolve()
            fields = capture.REQUIRED_TEXTURES | {"face_expression", "face_outline_control"}
            for resource in capture.MATERIAL_SCRIPTS:
                path = root / resource[6:]
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_text("\n".join("@export var " + name + ": Texture2D" for name in sorted(fields)))
            texture = root / "texture.png"
            texture.write_bytes(b"texture fixture")
            item = {"path": "res://texture.png", "sha256": capture.sha(texture)}
            snapshot = {name: dict(item) for name in fields}
            snapshot["face_expression"] = None
            capture.verify_textures(root, snapshot, dict(snapshot))
            omitted = dict(snapshot)
            omitted.pop("face_expression")
            with self.assertRaises(ValueError):
                capture.verify_textures(root, omitted, omitted)
            missing = {**snapshot, "face_base": None}
            with self.assertRaises(ValueError):
                capture.verify_textures(root, missing, missing)
            with self.assertRaises(ValueError):
                capture.verify_textures(root, snapshot, {**snapshot, "face_expression": item})
            texture.write_bytes(b"changed bytes")
            with self.assertRaises(ValueError):
                capture.verify_textures(root, snapshot, snapshot)


if __name__ == "__main__":
    unittest.main()
