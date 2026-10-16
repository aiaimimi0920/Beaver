import ast
import json
import math
from pathlib import Path
import unittest
from fit import fit_profile, inherited_curve
from curve_profile import delta, evaluate, eye_plane_delta, strength
from diagnostics import scope

RECIPE = Path(__file__).resolve().parents[1] / "checkpoints/current"


class CoherentTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.source = (RECIPE / "face.py").read_text()
        cls.c = json.loads((RECIPE / "face_style.json").read_text())["calibration"]
        cls.fit = fit_profile(cls.source, cls.c)
        cls.curve = staticmethod(inherited_curve(cls.source))

    def test_profile_c2_and_bound(self):
        fit, c = self.fit, self.c
        self.assertLess(fit["certified_max_profile_offset_m"], 0.002)
        self.assertLess(fit["boundary_jet_max_error"], 1e-8)
        for left, right in zip(fit["segments"], fit["segments"][1:]):
            for d in range(3):
                self.assertAlmostEqual(
                    evaluate(left[1], [left], d),
                    evaluate(right[0], [right], d),
                    places=8,
                )
        for i in range(2001):
            h = 0.1 + 0.8 * i / 2000
            displacement = (
                abs(
                    evaluate(h, fit["segments"])
                    - self.curve(h, 1, c["continuous_front_profile"])
                )
                * c["height_m"]
            )
            self.assertLessEqual(
                displacement, fit["certified_max_profile_offset_m"] + 1e-10
            )

    def test_global_collar_and_symmetry(self):
        c = self.c
        for h in (0.1, 0.25, 0.4, 0.6, 0.8, 0.9):
            y = c["origin_y_m"] + h * c["height_m"]
            width = c["height_m"] * self.curve(h, 1, c["width_stations"])
            for q in (0.0, 0.2, 0.5, 0.85, 0.95):
                x = q * width
                self.assertEqual(delta(x, y, c, self.curve, self.fit, 0), 0)
                value = delta(x, y, c, self.curve, self.fit, 1)
                self.assertEqual(value, delta(-x, y, c, self.curve, self.fit, 1))
                self.assertLessEqual(abs(value), 0.002)
                if q >= 0.85 or h in (0.1, 0.9):
                    self.assertLess(abs(value), 1e-14)
        for value in (True, -1, 1.1, math.nan):
            with self.assertRaises(ValueError):
                strength(value)

    def test_orbital_support_and_iris_plane_are_coherent(self):
        c = self.c
        old, new = [scope(self.source, c, self.fit, a) for a in (0, 1)]
        for x in (0.02, 0.035, 0.05):
            for h in (0.4, 0.55, 0.65):
                y = c["origin_y_m"] + h * c["height_m"]
                self.assertAlmostEqual(
                    new["depth"](x, y) - old["depth"](x, y),
                    delta(x, y, c, self.curve, self.fit, 1),
                    places=13,
                )
        cx, cy = (
            c["height_m"] * c["iris_center_x"],
            c["origin_y_m"] + c["height_m"] * c["iris_center_y"],
        )
        plane = lambda x, y: eye_plane_delta(x, y, c, self.curve, self.fit, 1)
        self.assertAlmostEqual(
            plane(cx, cy), delta(cx, cy, c, self.curve, self.fit, 1), places=14
        )
        self.assertAlmostEqual(
            plane(cx + 0.01, cy) + plane(cx - 0.01, cy), 2 * plane(cx, cy), places=14
        )
        self.assertAlmostEqual(
            plane(cx, cy + 0.01) + plane(cx, cy - 0.01), 2 * plane(cx, cy), places=14
        )


if __name__ == "__main__":
    unittest.main()
