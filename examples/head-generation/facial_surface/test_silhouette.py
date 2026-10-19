"""Width controls preserve zero mode and bound the complete interpolant."""

import ast
import json
import math
from pathlib import Path
import unittest
from source_silhouette import apply_silhouette, bezier_segments, validate

ROOT = Path(__file__).parent.parent / "checkpoints/current"


class SilhouetteTests(unittest.TestCase):
    def setUp(self):
        self.style = json.loads((ROOT / "face_style.json").read_text())

    def test_zero_identity_and_no_mutation(self):
        before = json.dumps(self.style)
        for p in (None, {}, {"jaw_width": 0}):
            result, report = apply_silhouette(self.style, p)
            self.assertEqual(result, self.style)
            self.assertIsNone(report)
            self.assertIsNot(result, self.style)
        apply_silhouette(self.style, {"jaw_width": 0.02})
        self.assertEqual(before, json.dumps(self.style))

    def test_ear_envelope_delta_preserves_safety_guards(self):
        from source_silhouette import adapt_ear_envelope
        source = (ROOT / "ear_width.py").read_text()
        original = self.style["calibration"]["width_stations"]
        adapted = adapt_ear_envelope(source, original)
        compile(adapted, "ear_width", "exec")
        self.assertIn("v=(p.z+.021-Y0)/H", adapted)
        self.assertIn("w*height+silhouette_delta-current", adapted)
        for guard in ("max(-.003,min(.003", "assert not invalid", "anchors.append(mapping)", "abs(p.x)<.035"):
            self.assertIn(guard, adapted)
        with self.assertRaises(ValueError):
            adapt_ear_envelope(source.replace("w*height-current", "w*height"), original)

    def test_invalid_controls(self):
        for value in (True, None, "0", float("nan"), float("inf"), 0.041):
            with self.assertRaises(ValueError):
                validate({"jaw_width": value})
        for p in ([], {"wrong": 0}):
            with self.assertRaises(ValueError):
                validate(p)

    def test_axes_and_combination_budget(self):
        for name, value in (
            ("face_width", 0.01),
            ("jaw_width", 0.02),
            ("chin_width", 0.08),
        ):
            for sign in (-1, 1):
                changed, report = apply_silhouette(self.style, {name: sign * value})
                self.assertLessEqual(
                    report["continuous_half_width_change_bound_m"], 0.002
                )
                self.assertNotEqual(
                    changed["calibration"]["width_stations"],
                    self.style["calibration"]["width_stations"],
                )
        with self.assertRaises(ValueError):
            apply_silhouette(self.style, {"jaw_width": 0.04})

    def test_exact_curve_conversion_and_bound(self):
        node = next(
            n
            for n in ast.parse((ROOT / "face.py").read_text()).body
            if isinstance(n, ast.FunctionDef) and n.name == "curve"
        )
        ns = {}
        exec(compile(ast.Module(body=[node], type_ignores=[]), "curve", "exec"), ns)
        changed, report = apply_silhouette(self.style, {"jaw_width": -0.02})
        original = self.style["calibration"]["width_stations"]
        points = changed["calibration"]["width_stations"]
        for i, coefficients in enumerate(bezier_segments(points)):
            for step in range(101):
                t = step / 100
                h = points[i][0] + t * (points[i + 1][0] - points[i][0])
                value = sum(
                    math.comb(3, j) * t**j * (1 - t) ** (3 - j) * c
                    for j, c in enumerate(coefficients)
                )
                self.assertAlmostEqual(value, ns["curve"](h, 1, points), places=13)
                delta = (
                    abs(value - ns["curve"](h, 1, original))
                    * self.style["calibration"]["height_m"]
                )
                self.assertLessEqual(
                    delta, report["continuous_half_width_change_bound_m"]
                )

    def test_unrelated_calibration_unchanged(self):
        changed, _ = apply_silhouette(self.style, {"jaw_width": 0.02})
        changed["calibration"]["width_stations"] = self.style["calibration"][
            "width_stations"
        ]
        self.assertEqual(changed, self.style)


if __name__ == "__main__":
    unittest.main()
