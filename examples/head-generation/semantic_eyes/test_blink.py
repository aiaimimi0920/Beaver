"""CPU-only packaging and scalar closure contracts; actual poses require Beaver."""

import ast
import json
import tempfile
import unittest
from pathlib import Path
from bundle import build_request, REQUIRED, RECIPE
from parameters import adapt_style, validate
from blink_runtime import closure_delta, quadratic_minimum
from export_contract import validate as validate_export
from test_export_contract import export, SOURCE
from glb_affine import read_glb, write_glb


class BlinkTests(unittest.TestCase):
    def request(self, blink=False, **parameters):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            for name in REQUIRED:
                path = root / name
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_bytes(b"fixture")
            return build_request(
                dict(
                    {"supplemental_lashes": 0, "lid_fold": 0, "lash_contour_follow": 1},
                    **parameters
                ),
                "assets/aster/general_face_eyes_blink_test/",
                root,
                blink=blink,
            )

    def test_opt_in_and_retained_source(self):
        default = self.request()
        enabled = self.request(True)
        self.assertEqual(
            [x["path"] for x in default["outputs"]],
            [x["path"] for x in enabled["outputs"]],
        )
        strings = [
            x.value
            for x in ast.walk(ast.parse(enabled["code"]))
            if isinstance(x, ast.Constant) and isinstance(x.value, str)
        ]
        self.assertTrue(
            any(
                "def build_blink_shapes" in x and "verify_pose_orientation(obj)" in x
                for x in strings
            )
        )
        self.assertTrue(
            any('report["eyelid_closure"] = build_blink_shapes' in x for x in strings)
        )
        self.assertTrue(
            any('== ["MouthOpen", "blink.L", "blink.R"]' in x for x in strings)
        )
        strings = [
            x.value
            for x in ast.walk(ast.parse(default["code"]))
            if isinstance(x, ast.Constant) and isinstance(x.value, str)
        ]
        actual_definitions = []
        for source in strings:
            try:
                nodes = ast.walk(ast.parse(source))
            except SyntaxError:
                continue
            actual_definitions.extend(n.name for n in nodes if isinstance(n, ast.FunctionDef))
        self.assertNotIn("build_blink_shapes", actual_definitions)

    def test_reject_unsupported_components(self):
        for params in (
            {"supplemental_lashes": 1},
            {"lid_fold": 1},
            {"lash_contour_follow": 0},
        ):
            with self.assertRaisesRegex(ValueError, "Blink requires"):
                self.request(True, **params)
        with self.assertRaisesRegex(ValueError, "boolean"):
            self.request(1)

    def test_skin_closure_is_monotonic_inside_aperture(self):
        c = adapt_style(
            json.loads((RECIPE / "face_style.json").read_text()), validate({})
        )["calibration"]
        x = c["height_m"] * c["eye_center_x"]
        center = c["origin_y_m"] + c["height_m"] * c["eye_corner_y"]
        lower = center - c["height_m"] * c["eye_lower_arch"]
        upper = center + c["height_m"] * c["eye_upper_arch"]
        ys = [lower + (upper - lower) * i / 100 for i in range(101)]
        for weight in (0, 0.25, 0.5, 0.75, 1):
            posed = [y + weight * closure_delta(x, y, "skin", c) for y in ys]
            self.assertTrue(all(a < b for a, b in zip(posed, posed[1:])))
        self.assertEqual(closure_delta(100, center, "skin", c), 0)

    def test_upper_ink_column_floor_meets_closed_seam(self):
        c = adapt_style(
            json.loads((RECIPE / "face_style.json").read_text()), validate({})
        )["calibration"]
        for side in (-1, 1):
            for t in (-0.9, 0.0, 0.7, 0.94):
                x = side * c["height_m"] * (c["eye_center_x"] + c["eye_half_width"] * t)
                seam = (
                    c["origin_y_m"]
                    + c["height_m"] * (c["eye_corner_y"] + c["eye_corner_slope"] * t)
                    - 0.0015 * (1 - t * t)
                )
                floor = seam + 0.01
                delta = closure_delta(x, floor, "upper_ink", c)
                self.assertAlmostEqual(floor + delta, seam)
                self.assertAlmostEqual((floor + 0.002 + delta) - (floor + delta), 0.002)
        self.assertEqual(closure_delta(100, 0, "upper_ink", c), 0)

    def test_export_requires_explicit_blinks(self):
        doc, data = read_glb(export())
        mesh = doc["meshes"][0]
        mesh["extras"]["targetNames"] = ["MouthOpen", "blink.L", "blink.R"]
        mesh["weights"] = [0, 0, 0]
        doc["nodes"][0]["weights"] = [0, 0, 0]
        p = mesh["primitives"][0]
        p["targets"] *= 3
        raw = write_glb(doc, data)
        with self.assertRaises(ValueError):
            validate_export(raw, SOURCE)
        self.assertEqual(
            validate_export(raw, SOURCE, with_blinks=True)["morph_names"],
            mesh["extras"]["targetNames"],
        )
        p["targets"][2] = dict(p["targets"][2])
        p["targets"][2].pop("NORMAL")
        with self.assertRaises(ValueError):
            validate_export(write_glb(doc, data), SOURCE, with_blinks=True)
        with self.assertRaises(ValueError):
            validate_export(export(), SOURCE, with_blinks=True)

    def test_quadratic_minimum_bounds_dense_samples(self):
        import random

        random.seed(7)
        for _ in range(100):
            a, b, c, d, e, f = [random.uniform(-5, 5) for _ in range(6)]
            minimum = quadratic_minimum(a, b, c, d, e, f)[0]
            grid = min(
                a + b * x + c * y + d * x * x + e * x * y + f * y * y
                for x in [i / 20 for i in range(21)]
                for y in [i / 20 for i in range(21)]
            )
            self.assertLessEqual(minimum, grid + 1e-12)

    def test_quadratic_interior_and_boundary(self):
        value, x, y = quadratic_minimum(0.13, -0.4, -0.6, 1, 0, 1)
        self.assertAlmostEqual(value, 0)
        self.assertAlmostEqual(x, 0.2)
        self.assertAlmostEqual(y, 0.3)
        self.assertAlmostEqual(quadratic_minimum(1, -3, 0, 1, 0, 1)[0], -1)
        self.assertEqual(quadratic_minimum(1, 0, 0, 0, -3, 0)[0], -2)


if __name__ == "__main__":
    unittest.main()
