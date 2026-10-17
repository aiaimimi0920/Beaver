"""Bounded iris-only controls; no native model or appearance acceptance."""

import ast
import copy
import json
from pathlib import Path
import unittest

from parameters import adapt_style, validate


class IrisHorizontalOffsetTests(unittest.TestCase):
    def setUp(self):
        self.recipe = Path(__file__).parents[1] / "checkpoints/current"
        self.base = json.loads((self.recipe / "face_style.json").read_text())

    def test_neutral_identity_and_numeric_bounds(self):
        before = copy.deepcopy(self.base)
        self.assertEqual(adapt_style(self.base, {}), before)
        self.assertEqual(adapt_style(self.base, {"iris_horizontal_offset": 0}), before)
        self.assertEqual(validate({})["ocular_skin_outline_holdout"], 0)
        for value in (-0.012, 0.012):
            self.assertEqual(
                validate({"iris_horizontal_offset": value})["iris_horizontal_offset"],
                value,
            )
        for value in (-0.012001, 0.012001, True, float("nan"), float("inf")):
            with self.assertRaises(ValueError):
                validate({"iris_horizontal_offset": value})
        for value in (-1, 0.5, 2, True):
            with self.assertRaises(ValueError):
                validate({"ocular_skin_outline_holdout": value})
        self.assertEqual(self.base, before)

    def test_signed_shift_after_width_mapping_preserves_other_calibration(self):
        for width in (0.92, 1.0, 1.08):
            request = {"eye_width": width, "brow_thickness": 0.99}
            reference = adapt_style(self.base, request)
            for offset in (-0.012, -0.006, 0.006, 0.012):
                result = adapt_style(
                    self.base, dict(request, iris_horizontal_offset=offset)
                )
                old, new = reference["calibration"], result["calibration"]
                self.assertAlmostEqual(
                    new["iris_center_x"] - old["iris_center_x"], offset
                )
                for key in old:
                    if key != "iris_center_x":
                        self.assertEqual(new[key], old[key], key)
                result["calibration"]["iris_center_x"] = old["iris_center_x"]
                self.assertEqual(result, reference)
                for side in (-1, 1):
                    delta = side * old["height_m"] * offset
                    self.assertEqual(delta < 0, (side > 0) == (offset < 0))

    def test_mask_control_is_independent_and_explicit(self):
        marked = dict(self.base, illustrated_eye_art=True)
        reference = adapt_style(marked, {})
        enabled = adapt_style(marked, {"ocular_skin_outline_holdout": 1})
        self.assertTrue(enabled.pop("ocular_skin_outline_holdout"))
        self.assertEqual(enabled, reference)
        stale = dict(marked, ocular_skin_outline_holdout=True)
        self.assertEqual(
            adapt_style(stale, {"ocular_skin_outline_holdout": 0}), reference
        )
        gaze = adapt_style(marked, {"iris_horizontal_offset": -0.006})
        self.assertNotIn("ocular_skin_outline_holdout", gaze)

    def test_retained_native_configs_isolate_translation(self):
        folder = self.recipe.parent.parent / "facial_surface"
        baseline = json.loads((folder / "emerald-soft-aperture-eyes.json").read_text())
        nasal = json.loads((folder / "emerald-nasal-gaze-eyes.json").read_text())
        self.assertEqual(validate(baseline)["iris_plane_slope_scale"], 1)
        self.assertEqual(validate(nasal)["ocular_skin_outline_holdout"], 0)
        self.assertEqual(nasal.pop("iris_horizontal_offset"), -0.006)
        self.assertEqual(baseline.pop("iris_horizontal_offset"), 0)
        self.assertEqual(nasal, baseline)

    def test_actual_geometry_centers_follow_shared_iris_calibration(self):
        # Extract production center assignments rather than duplicating its formulas.
        tree = ast.parse((self.recipe / "eye_socket.py").read_text())
        targets = {
            "annular_iris_mesh": "center_x",
            "build_pupil_layer": "x",
            "build_catchlight": "x",
        }
        for function_name, target in targets.items():
            function = next(
                n
                for n in tree.body
                if isinstance(n, ast.FunctionDef) and n.name == function_name
            )
            assignment = next(
                n
                for n in ast.walk(function)
                if isinstance(n, ast.Assign)
                and isinstance(n.targets[0], ast.Name)
                and n.targets[0].id == target
            )
            code = compile(
                ast.Expression(assignment.value), "production-center", "eval"
            )
            for side in (-1, 1):
                baseline = self.base["calibration"]
                shifted = adapt_style(self.base, {"iris_horizontal_offset": -0.006})[
                    "calibration"
                ]

                def evaluate(c):
                    return eval(
                        code,
                        {"__builtins__": {}},
                        dict(C=c, H=c["height_m"], side=side, rx=0.01, u=0.3),
                    )

                self.assertAlmostEqual(
                    evaluate(shifted) - evaluate(baseline),
                    side * baseline["height_m"] * (-0.006),
                )


if __name__ == "__main__":
    unittest.main()
