"""Profile scope and helper retention in the actual CPU-built request."""

import ast
import json
from pathlib import Path
import tempfile
import unittest
from bundle import build_request


class ProfilePackagingTests(unittest.TestCase):
    def test_layered_scope_helpers_and_output_budget(self):
        here = Path(__file__).parent
        eyes = json.loads((here / "emerald-layered-eyes.json").read_text())
        with tempfile.TemporaryDirectory() as workspace:
            root = Path(workspace)
            paths = ["project.godot", "authoring/head_generation_guide.md"] + [
                "assets/aster/head_recovery_49/" + name
                for name in (
                    "aster_head_editable.blend",
                    "aster_head.glb",
                    "aster_definition.tres",
                    "hair_base.png",
                    "hair_ilm.png",
                )
            ]
            for name in paths:
                path = root / name
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_text("CPU fixture")
            request = build_request(
                {
                    "axial_smoothing": 0.5,
                    "cheek_roundness": 0.05,
                    "orbit_shallowing": 0.25,
                },
                eyes,
                "assets/aster/general_face_eyes_cpu/",
                Path(workspace),
                0.0042,
                "emerald_layered",
            )
        outputs = [v["path"] for v in request["outputs"]]
        self.assertEqual(len(outputs), 64)
        self.assertEqual(len(set(outputs)), 64)
        source = request["code"]
        scope_line = next(
            line
            for line in source.splitlines()
            if line.startswith('report["semantic_eyes"]["scope"] =')
        )
        self.assertIn(
            "nose-plane pigment", ast.literal_eval(scope_line.split(" = ", 1)[1])
        )
        tree = ast.parse(source)
        payloads = {}
        for node in ast.walk(tree):
            if (
                isinstance(node, ast.Call)
                and isinstance(node.func, ast.Attribute)
                and node.func.attr == "write_text"
                and node.args
            ):
                value = node.args[0]
                if isinstance(value, ast.Constant) and isinstance(value.value, str):
                    try:
                        data = json.loads(value.value)
                    except ValueError:
                        continue
                    if isinstance(data, dict):
                        payloads.update(data)
        helpers = payloads["packager_helpers"]
        self.assertEqual(
            helpers["source_styles.py"], (here / "source_styles.py").read_text()
        )
        self.assertIn("nose-plane pigment", payloads["report"]["style_profile_scope"])


if __name__ == "__main__":
    unittest.main()
