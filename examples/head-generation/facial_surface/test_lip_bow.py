"""Independent lip ridge family keeps rest aperture and bounds analytic relief."""

import ast
import json
import math
from pathlib import Path
import unittest
from source_lips import apply_lip_relief, adapt_lip_source
from interval_bounds import raw_without_origin, I
from study import scope, fit_profile

RECIPE = Path(__file__).parents[1] / "checkpoints/current"
SOURCE = (RECIPE / "face.py").read_text()


def lip_function(source, c):
    nodes = [
        n
        for n in ast.parse(source).body
        if isinstance(n, ast.FunctionDef)
        and n.name in ("lip_detail_depth", "lip_shoulder_support")
    ]
    env = {"H": c["height_m"], "Y0": c["origin_y_m"], "C": c, "exp": math.exp}
    exec(compile(ast.Module(body=nodes, type_ignores=[]), "<lip test>", "exec"), env)
    return env["lip_detail_depth"]


class BowTests(unittest.TestCase):
    def setUp(self):
        self.style = json.loads((RECIPE / "face_style.json").read_text())

    def test_identity(self):
        changed, report = apply_lip_relief(
            self.style, {"lower_lip_relief_scale": 1, "upper_lip_bow_height": 0}
        )
        self.assertEqual(changed, self.style)
        self.assertIsNone(report)
        self.assertEqual(adapt_lip_source(SOURCE, changed["calibration"]), SOURCE)

    def test_seam_and_other_controls_unchanged(self):
        changed, _ = apply_lip_relief(
            self.style, {"lower_lip_relief_scale": 1.15, "upper_lip_bow_height": 0.002}
        )
        a = self.style["calibration"]
        b = changed["calibration"]
        self.assertEqual({k for k in a if a[k] != b[k]}, {"lower_lip_relief"})
        self.assertEqual(set(b) - set(a), {"upper_lip_bow_height"})
        self.assertIn("mouth_seam_recess", adapt_lip_source(SOURCE, b))

    def test_delta_bound(self):
        changed, report = apply_lip_relief(
            self.style,
            {
                "upper_lip_relief_scale": 0.95,
                "lower_lip_relief_scale": 1.25,
                "upper_lip_bow_height": 0.003,
            },
        )
        a = self.style["calibration"]
        b = changed["calibration"]
        f = lip_function(SOURCE, a)
        g = lip_function(adapt_lip_source(SOURCE, b), b)
        for ix in range(-30, 31):
            for iy in range(61):
                x = ix * 0.001
                y = a["origin_y_m"] + a["height_m"] * (0.06 + iy * 0.003)
                self.assertLessEqual(
                    abs(f(x, y) - g(x, y)), report["analytic_depth_change_bound_m"]
                )

    def test_combined_lateral_power_and_maximum_bow_bound(self):
        changed, report = apply_lip_relief(
            self.style,
            {
                "upper_lip_relief_scale": 0.85,
                "upper_lip_lateral_power": 2,
                "lower_lip_relief_scale": 1.25,
                "upper_lip_bow_height": 0.003,
            },
        )
        a = self.style["calibration"]
        b = changed["calibration"]
        f = lip_function(SOURCE, a)
        g = lip_function(adapt_lip_source(SOURCE, b), b)
        for ix in range(-40, 41):
            for iy in range(51):
                x = ix * 0.001
                y = a["origin_y_m"] + a["height_m"] * (0.05 + iy * 0.005)
                self.assertLessEqual(
                    abs(f(x, y) - g(x, y)), report["analytic_depth_change_bound_m"]
                )
        self.assertEqual(report["parameters"]["lower_lip_relief_scale"], 1.25)
        self.assertEqual(report["parameters"]["upper_lip_bow_height"], 0.003)

    def test_interval_covers_bowed_source(self):
        changed, _ = apply_lip_relief(
            self.style, {"upper_lip_bow_height": 0.003, "lower_lip_relief_scale": 1.25}
        )
        c = changed["calibration"]
        source = adapt_lip_source(SOURCE, c)
        env = scope(source, c, fit_profile(source, c), 0)
        for ix in range(-8, 9):
            for iy in range(12):
                x = ix * 0.002
                h = 0.09 + iy * 0.01
                bound = raw_without_origin(I(x), I(h), c)
                # Raw field source uses Z0; remove exactly that authored origin.
                value = (
                    env["base_depth"](x, c["origin_y_m"] + c["height_m"] * h)
                    - env["Z0"]
                )
                self.assertLessEqual(bound.lo - 1e-9, value)
                self.assertGreaterEqual(bound.hi + 1e-9, value)

    def test_invalid(self):
        for p in (
            {"lower_lip_relief_scale": 1.26},
            {"lower_lip_relief_scale": True},
            {"upper_lip_bow_height": float("nan")},
            {"upper_lip_bow_height": -0.001},
        ):
            with self.assertRaises(ValueError):
                apply_lip_relief(self.style, p)


if __name__ == "__main__":
    unittest.main()
