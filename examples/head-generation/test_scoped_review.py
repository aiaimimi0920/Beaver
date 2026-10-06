"""Regression limits for the five-feature review after candidate145."""

import json
import unittest
from pathlib import Path
from test_style_math import pure_function

STYLE = json.loads(Path(__file__).with_name("face_style.json").read_text())
C = STYLE["calibration"]


class ScopedReviewTests(unittest.TestCase):
    def test_nasal_tip_anchor_and_mouth_follow_stay_bounded(self):
        curve, _ = pure_function("face.py", "curve", {})
        self.assertAlmostEqual(curve(0.28, 1, C["nose_stations"]), 0.0945)
        self.assertGreater(curve(0.32, 1, C["nose_stations"]), 0.066)
        self.assertLessEqual(curve(0.4, 1, C["continuous_front_profile"]), 0.066)
        self.assertAlmostEqual(C["chin_follow_down_m"], 0.0015)

    def test_local_contour_support_and_ear_return_constraints(self):
        curve, _ = pure_function("face.py", "curve", {})
        self.assertGreater(curve(0.15, 1, C["width_stations"]), 0.267)
        self.assertLess(curve(0.15, 1, C["width_stations"]), 0.280)
        self.assertLessEqual(C["ear_return_basin_depth_fraction"], 0.25)
        self.assertLessEqual(C["ear_return_basin_width_fraction"], 0.5)
        self.assertGreater(C["ear_outward_span"], 4 * C["ear_cup_recess"])

    def test_brow_pupil_and_connected_lash_contract(self):
        self.assertLessEqual(C["brow_thickness"], 0.011)
        self.assertTrue(0.7 < STYLE["iris_pupil_aspect"] < 0.9)
        self.assertTrue(0.20 < STYLE["iris_pupil_radius"] < 0.26)
        code = Path(__file__).with_name("face.py").read_text()
        self.assertIn("Attached outer lash wing", code)
        self.assertNotIn("Attached lash accent", code)
        self.assertNotIn("Soft outer lash flick", code)


class SideFairingSeparationTests(unittest.TestCase):
    def test_side_fairing_preserves_front_width(self):
        fn, _ = pure_function("shell.py", "fair_jaw_boundary", {})
        vs = [
            (0.301, 0.30, 0.145),
            (0.302, 0.301, 0.15),
            (0.300, 0.302, 0.155),
            (0.299, 0.30, 0.16),
        ]
        out, report = fn(vs, [(0, 1, 2, 3)], C, 1, 0, 0)
        self.assertEqual([v[0] for v in out], [v[0] for v in vs])
        self.assertGreater(report["maximum_displacement_m"], 0)
