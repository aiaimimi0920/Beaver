"""Bounded eyelid paint preserves white space below the occlusion band."""

import math
import unittest
from illustrated_art import IRIS


class ScleraSoftPaintTests(unittest.TestCase):
    def setUp(self):
        self.ns = dict(vars(math))
        self.ns.update(
            WHITE=(0.95, 0.94, 0.92),
            Y0=0.0,
            H=1.0,
            C={"eye_corner_y": 0.0},
            eye_contour=lambda side, t, upper: (t, 0.1),
        )
        exec(IRIS, self.ns)

    def test_occlusion_is_restrained_and_does_not_fill_lower_white(self):
        paint = self.ns["sclera_art_color"]
        upper = paint(0.5, 0.7)
        lower = paint(0.5, 0.3)
        self.assertGreater(sum(upper[:3]), 2.5)
        self.assertLess(sum(upper[:3]), sum(lower[:3]))
        for got, expected in zip(lower[:3], self.ns["WHITE"]):
            self.assertAlmostEqual(got, expected, places=8)

    def test_shadow_band_does_not_extend_below_reviewed_depth(self):
        # y=.07 is .03 below the synthetic upper lid at .10.
        # The previous .045-H band shaded this sample; the .025-H band must not.
        color = self.ns["sclera_art_color"](0.5, 0.64)
        for got, expected in zip(color[:3], self.ns["WHITE"]):
            self.assertAlmostEqual(got, expected, places=8)

    def test_symmetry_and_no_new_alpha_holes(self):
        paint = self.ns["sclera_art_color"]
        for u in [0.0, 0.1, 0.3, 0.5]:
            for v in [0.0, 0.3, 0.5, 0.7, 1.0]:
                a, b = paint(u, v), paint(1 - u, v)
                for x, y in zip(a, b):
                    self.assertAlmostEqual(x, y, places=12)
                self.assertEqual(a[3], 1)
