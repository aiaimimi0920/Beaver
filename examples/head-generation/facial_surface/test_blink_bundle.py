"""CPU-only opt-in blink packaging must fit the native request contract."""

import ast
import json
from pathlib import Path
import tempfile
import unittest

from bundle import build_request


class BlinkBundleTests(unittest.TestCase):
    def test_blink_retention_and_native_limits(self):
        here = Path(__file__).parent
        eyes = json.loads((here / "emerald-layered-eyes.json").read_text())
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            inputs = ["project.godot", "authoring/head_generation_guide.md"] + [
                "assets/aster/head_recovery_49/" + name
                for name in (
                    "aster_head_editable.blend",
                    "aster_head.glb",
                    "aster_definition.tres",
                    "hair_base.png",
                    "hair_ilm.png",
                )
            ]
            for name in inputs:
                path = root / name
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_bytes(b"CPU fixture")
            request = build_request(
                {
                    "axial_smoothing": 0.5,
                    "cheek_roundness": 0.05,
                    "orbit_shallowing": 0.25,
                },
                eyes,
                "assets/aster/general_face_eyes_blink_cpu/",
                root,
                0.0042,
                "emerald_layered",
                blink=True,
            )
        paths = [entry["path"] for entry in request["outputs"]]
        self.assertEqual(len(paths), 64)
        self.assertEqual(len({path.casefold() for path in paths}), 64)
        self.assertLessEqual(len(request["code"].encode("utf-8")), 1024 * 1024)
        tree = ast.parse(request["code"])
        retained = {}
        for node in ast.walk(tree):
            if (
                isinstance(node, ast.Call)
                and isinstance(node.func, ast.Attribute)
                and node.func.attr == "write_text"
                and node.args
                and isinstance(node.args[0], ast.Constant)
                and isinstance(node.args[0].value, str)
            ):
                destination = node.func.value
                if isinstance(destination, ast.Call) and destination.args:
                    output = destination.args[0]
                    if isinstance(output, ast.Call) and output.args:
                        path = output.args[0]
                        if isinstance(path, ast.Constant) and isinstance(
                            path.value, str
                        ):
                            retained[Path(path.value).name] = node.args[0].value
        runtime = retained["semantic_eye_runtime.py"]
        self.assertIn("def build_blink_shapes", runtime)
        self.assertIn("def verify_pose_orientation", runtime)
        self.assertIn(
            'report["eyelid_closure"] = build_blink_shapes', retained["pipeline.py"]
        )
        self.assertIn(
            '== ["MouthOpen", "blink.L", "blink.R"]', retained["glb_merge.py"]
        )


if __name__ == "__main__":
    unittest.main()
