"""Silhouette analysis and native recipe share one final calibration."""

import ast
import hashlib
import json
from pathlib import Path
import tempfile
import unittest
from bundle import build_request
from source_silhouette import apply_silhouette
from source_styles import apply_style_profile

HERE = Path(__file__).parent


class SilhouetteBundleTests(unittest.TestCase):
    def test_native_analysis_and_provenance_agree(self):
        style = json.loads(
            (HERE.parent / "checkpoints/current/face_style.json").read_text()
        )
        expected, report = apply_silhouette(
            apply_style_profile(style, "emerald_illustrated"), {"jaw_width": 0.02}
        )
        eyes = json.loads(
            (HERE.parent / "eye_presets/clean-downturned-810.json").read_text()
        )["eye_parameters"]
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            required = ["project.godot", "authoring/head_generation_guide.md"] + [
                "assets/aster/head_recovery_49/" + f
                for f in (
                    "aster_head_editable.blend",
                    "aster_head.glb",
                    "aster_definition.tres",
                    "hair_base.png",
                    "hair_ilm.png",
                )
            ]
            for name in required:
                path = root / name
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_text("CPU fixture")
            request = build_request(
                {
                    "axial_smoothing": 0.5,
                    "cheek_roundness": 0.05,
                    "orbit_shallowing": 0.15,
                },
                eyes,
                "assets/aster/general_face_eyes_silhouette_cpu/",
                root,
                0.0042,
                "emerald_illustrated",
                blink=True,
                silhouette_parameters={"jaw_width": 0.02},
            )
        payloads = []

        # Retained source includes nested JSON strings; inspect constants recursively.
        def visit(source, depth=0):
            if depth > 3:
                return
            try:
                tree = ast.parse(source)
            except SyntaxError:
                return
            for node in ast.walk(tree):
                if isinstance(node, ast.Constant) and isinstance(node.value, str):
                    try:
                        data = json.loads(node.value)
                        if isinstance(data, dict):
                            payloads.append(data)
                    except ValueError:
                        pass
                    if "import " in node.value:
                        visit(node.value, depth + 1)

        visit(request["code"])
        calibrations = [p["calibration"] for p in payloads if "calibration" in p]
        self.assertTrue(calibrations)
        for c in calibrations:
            self.assertEqual(
                c["width_stations"], expected["calibration"]["width_stations"]
            )
        preflight = next(p for p in payloads if "packager_helpers" in p)
        self.assertEqual(preflight["report"]["silhouette"], report)
        helper = (HERE / "source_silhouette.py").read_text()
        self.assertEqual(preflight["packager_helpers"]["source_silhouette.py"], helper)
        manifest = next(p for p in payloads if "facial_surface_preflight.json" in p)
        serialized = json.dumps(preflight, indent=2)
        self.assertEqual(
            manifest["facial_surface_preflight.json"],
            hashlib.sha256(serialized.encode()).hexdigest(),
        )
        self.assertLessEqual(len(request["outputs"]), 64)

    def test_preserved_prefix_rejected(self):
        for n in (795, 796):
            with self.assertRaisesRegex(ValueError, "fresh output"):
                build_request(
                    {},
                    {},
                    f"assets/aster/general_face_eyes_{n}/",
                    Path("."),
                    0.0042,
                    silhouette_parameters={"jaw_width": 0.02},
                )


if __name__ == "__main__":
    unittest.main()
