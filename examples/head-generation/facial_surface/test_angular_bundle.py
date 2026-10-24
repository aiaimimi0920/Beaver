"""The facial package must prepare and emit the same upper shape."""

import ast
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch
import bundle


def retained_sources(code):
    result = {}
    for node in ast.walk(ast.parse(code)):
        if not (
            isinstance(node, ast.Call)
            and isinstance(node.func, ast.Attribute)
            and node.func.attr == "write_text"
            and node.args
            and isinstance(node.args[0], ast.Constant)
            and isinstance(node.args[0].value, str)
        ):
            continue
        dest = node.func.value
        if isinstance(dest, ast.Call) and dest.args:
            call = dest.args[0]
            if (
                isinstance(call, ast.Call)
                and call.args
                and isinstance(call.args[0], ast.Constant)
            ):
                result[Path(call.args[0].value).name] = node.args[0].value
    return result


class AngularBundleTests(unittest.TestCase):
    def test_preflight_native_and_retained_helper_agree(self):
        for amount in (0.25, 0.75):
            with self.subTest(amount=amount):
                self.check_preflight_native_and_retained_helper(amount)

    def check_preflight_native_and_retained_helper(self, amount):
        eyes = json.loads(
            (Path(__file__).parent / "emerald-balanced-layout-eyes.json").read_text()
        )
        eyes.update(
            upper_angularity=amount,
            lash_contour_follow=1,
            corner_slope_offset=-0.014,
            lower_arch=0.7,
            lower_roundness=1.25,
        )
        names = ["project.godot", "authoring/head_generation_guide.md"] + [
            "assets/aster/head_recovery_49/" + n
            for n in (
                "aster_head_editable.blend",
                "aster_head.glb",
                "aster_definition.tres",
                "hair_base.png",
                "hair_ilm.png",
            )
        ]
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            for name in names:
                path = root / name
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_bytes(b"CPU fixture")
            with patch.object(bundle, "prepare", wraps=bundle.prepare) as spy:
                request = bundle.build_request(
                    {
                        "axial_smoothing": 0.5,
                        "cheek_roundness": 0.05,
                        "orbit_shallowing": 0.15,
                    },
                    eyes,
                    "assets/aster/general_face_eyes_warp_cpu/",
                    root,
                    0.0042,
                    "emerald_illustrated",
                    blink=True,
                )
            args = spy.call_args.args
            self.assertIn("def eye_warp(", args[0])
            self.assertEqual(args[1]["eye_upper_angularity"], amount)
        retained = retained_sources(request["code"])
        self.assertIn("def eye_warp(", retained["face.py"])
        self.assertIn("def eye_warp(", retained["semantic_eye_runtime.py"])
        self.assertIn("eye_warp(t, C)", retained["eye_socket.py"])
        self.assertIn("upper_shape(t,c)", retained["lateral_fit.py"])
        scope = {}
        exec(
            compile(
                retained["semantic_parameters.py"],
                "<frozen numeric parameters>",
                "exec",
            ),
            scope,
        )
        self.assertEqual(scope["validate"](eyes)["upper_angularity"], amount)
        ast_tree = ast.parse(retained["semantic_bundle.py"])
        embedded = next(
            ast.literal_eval(n.value)
            for n in ast_tree.body
            if isinstance(n, ast.Assign)
            and any(
                isinstance(t, ast.Name) and t.id == "RETAINED_UPPER_SHAPE_SOURCES"
                for t in n.targets
            )
        )
        self.assertEqual(
            set(embedded),
            {
                "contour_warp.py",
                "source_upper_warp.py",
                "source_lateral_rim.py",
                "source_angular_samples.py",
                "sampling_contract.py",
                "source_density_layout.py",
                "source_cage_fairing.py",
                "source_outline_sampling.py",
                "source_outline_stage.py",
                "source_return_order.py",
                "source_lateral_regularization.py",
                    "source_brow_profiles.py",
            },
        )
        self.assertEqual(len(request["outputs"]), 64)
        self.assertLessEqual(len(request["code"].encode()), 1024 * 1024)


if __name__ == "__main__":
    unittest.main()
