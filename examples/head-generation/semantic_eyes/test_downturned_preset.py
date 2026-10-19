"""Real tilted aperture propagation, not a visual claim from a parameter name."""

import ast
import json
from pathlib import Path
import unittest
from bundle import RECIPE
from parameters import adapt_style, validate
from test_lower_contour import contour
from blink_runtime import closure_delta


class DownturnedPresetTests(unittest.TestCase):
    def setUp(self):
        self.style = json.loads((RECIPE / "face_style.json").read_text())
        self.eyes = json.loads(
            (
                Path(__file__).parents[1] / "eye_presets" / "soft-downturned-798.json"
            ).read_text()
        )["eye_parameters"]

    def test_range_default_and_existing_warp_guard(self):
        self.assertEqual(adapt_style(self.style, {}), self.style)
        self.assertEqual(
            validate({"corner_slope_offset": -0.030})["corner_slope_offset"], -0.030
        )
        for value in (-0.030001, 0.008001, float("nan"), True):
            with self.assertRaises(ValueError):
                validate({"corner_slope_offset": value})
        for key in ("upper_asymmetry", "upper_angularity", "lower_shoulder"):
            with self.assertRaises(ValueError):
                adapt_style(
                    self.style,
                    dict(self.eyes, corner_slope_offset=-0.026, **{key: 0.1}),
                )

    def test_806_real_contour_mirror_delta_and_closed_seam(self):
        old = adapt_style(self.style, self.eyes)["calibration"]
        new = adapt_style(self.style, dict(self.eyes, corner_slope_offset=-0.026))[
            "calibration"
        ]
        before, after = contour(old), contour(new)
        self.assertAlmostEqual(new["eye_corner_slope"], -0.012)
        for side in (-1, 1):
            self.assertAlmostEqual(
                after(side, -1, True)[1] - after(side, 1, True)[1], 0.005664
            )
            for i in range(101):
                t = -1 + i / 50
                for upper in (True, False):
                    x, y = after(side, t, upper)
                    self.assertAlmostEqual(
                        y - before(side, t, upper)[1], new["height_m"] * (-0.008) * t
                    )
                    self.assertEqual(after(-side, t, upper), (-x, y))
                    seam = (
                        new["origin_y_m"]
                        + new["height_m"]
                        * (new["eye_corner_y"] + new["eye_corner_slope"] * t)
                        - 0.0015 * (1 - t * t)
                    )
                    self.assertAlmostEqual(
                        y + closure_delta(x, y, "upper" if upper else "lower", new),
                        seam,
                    )

    def test_real_inverse_round_trip_and_monotone_outer_branches(self):
        source = (RECIPE / "lateral_fit.py").read_text()
        node = next(
            n
            for n in ast.parse(source).body
            if isinstance(n, ast.FunctionDef) and n.name == "eye_rim_x_at_height"
        )
        scope = {}
        exec(
            compile(
                ast.Module(body=[node], type_ignores=[]), "<actual inverse>", "exec"
            ),
            scope,
        )
        for offset in (-0.018, -0.026, -0.030):
            c = adapt_style(self.style, dict(self.eyes, corner_slope_offset=offset))[
                "calibration"
            ]
            curve = contour(c)
            for upper in (True, False):
                previous = curve(1, 0.4, upper)[1]
                for i in range(1, 101):
                    t = 0.4 + 0.6 * i / 100
                    x, y = curve(1, t, upper)
                    (
                        self.assertLess(y, previous)
                        if upper
                        else self.assertGreater(y, previous)
                    )
                    self.assertAlmostEqual(
                        scope["eye_rim_x_at_height"](y, c), x, places=11
                    )
                    previous = y

    def test_selected_lash_controls_and_fold_slope_delta(self):
        old = adapt_style(self.style, self.eyes)
        new = adapt_style(self.style, dict(self.eyes, corner_slope_offset=-0.026))
        slope_scale = old["accent_trajectory_slope_scale"]
        for key in ("upper_lash_rails", "inner_lash_fork_outline"):
            self.assertEqual(len(old[key]), len(new[key]))
            for a, b in zip(old[key], new[key]):
                self.assertEqual(a[0], b[0])
                # Hermite tail uses an independently fixed tip derivative.
                if a[0] <= 0.70 or (key == "upper_lash_rails" and a[0] >= 1.18):
                    self.assertAlmostEqual(
                        b[1] - a[1], -0.008 * a[0] * (1 - slope_scale), places=10
                    )
        for a, b in zip(old["lid_fold_height_profile"], new["lid_fold_height_profile"]):
            self.assertAlmostEqual(b[1] - a[1], -0.008 * (-0.76 + 1.52 * a[0]))
