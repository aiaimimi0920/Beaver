import unittest
import json
from pathlib import Path
from test_style_math import pure_function

C = json.loads(Path(__file__).with_name("face_style.json").read_text())["calibration"]


class ContourFeedbackTests(unittest.TestCase):

    def test_front_face_is_not_changed_by_return_band(self):
        curve, _ = pure_function("face.py", "curve", {})
        fn, _ = pure_function(
            "shell.py",
            "jaw_return_position",
            {"C": C, "H": 0.236, "Y0": 1.544, "Z0": 0.078, "curve": curve},
        )
        for q in [0, 0.3, 0.6, 0.78]:
            self.assertEqual(fn(0.04, 1.58, 0.082, q, 0.04), (0.04, 0.082))
        self.assertEqual(fn(0.04, 1.73, 0.08, 1, 0.04), (0.04, 0.08))

    def test_lower_return_is_inward_and_finite(self):
        curve, _ = pure_function("face.py", "curve", {})
        fn, _ = pure_function(
            "shell.py",
            "jaw_return_position",
            {"C": C, "H": 0.236, "Y0": 1.544, "Z0": 0.078, "curve": curve},
        )
        x, z = fn(0.236 * 0.18, 1.544 + 0.236 * 0.08, 0.04, 1, 0.04)
        self.assertLess(x, 0.236 * 0.18)
        self.assertGreater(x, 0)
        self.assertAlmostEqual(z, 0.078 - 0.236 * 0.25)

    def test_existing_chin_return_base_remains_unchanged(self):
        curve, _ = pure_function("face.py", "curve", {})
        fn, _ = pure_function(
            "shell.py",
            "jaw_return_position",
            {"C": C, "H": 0.236, "Y0": 1.544, "Z0": 0.078, "curve": curve},
        )
        for height in [0, 0.007, 0.015, 0.02]:
            self.assertEqual(fn(0.008, 1.544 + 0.236 * height, 0.072, 1, 0.04), (0.008, 0.072))

    def test_lateral_return_keeps_positive_parameter_order(self):
        curve, _ = pure_function("face.py", "curve", {})
        fn, _ = pure_function(
            "shell.py",
            "jaw_return_position",
            {"C": C, "H": 0.236, "Y0": 1.544, "Z0": 0.078, "curve": curve},
        )
        for v in [0.04, 0.08, 0.12, 0.2, 0.26, 0.4]:
            width = 0.236 * curve(v, 1, C["width_stations"])
            xs = [
                fn(width * i / 100, 1.544 + 0.236 * v, 0.04, i / 100, 0.04)[0] for i in range(101)
            ]
            self.assertTrue(all((b > a for a, b in zip(xs, xs[1:]))))

    def test_jaw_adjustment_does_not_reach_temple_or_eye_patch(self):
        curve, _ = pure_function("face.py", "curve", {})
        fn, _ = pure_function(
            "shell.py",
            "jaw_return_position",
            {"C": C, "H": 0.236, "Y0": 1.544, "Z0": 0.078, "curve": curve},
        )
        for v in [(1.614 - 1.544) / 0.236, 0.36, 0.4, 0.45, 0.52, 0.6]:
            for q in [0.78, 0.85, 0.94, 1]:
                self.assertEqual(fn(0.08, 1.544 + 0.236 * v, 0.01, q, 0.04), (0.08, 0.01))

    def test_return_translates_existing_cross_section_not_flattens_it(self):
        curve, _ = pure_function("face.py", "curve", {})
        fn, _ = pure_function(
            "shell.py",
            "jaw_return_position",
            {"C": C, "H": 0.236, "Y0": 1.544, "Z0": 0.078, "curve": curve},
        )
        a = fn(0.04, 1.58, 0.05, 0.9, 0.04)[1]
        b = fn(0.04, 1.58, 0.06, 0.9, 0.04)[1]
        self.assertAlmostEqual(b - a, 0.01)

    def test_outer_ear_depth_is_independent_of_root_waviness(self):
        curve, _ = pure_function("face.py", "curve", {})
        fn, _ = pure_function("ears.py", "membrane_offset", {"curve": curve})
        for t in [0.1, 0.3, 0.55, 0.78, 0.94]:
            target = curve(t, 1, C["ear_outer_depth_profile"])
            for root in [-0.30, -0.33, -0.36]:
                _, posterior = fn(t, 1, C, root)
                self.assertAlmostEqual(root - posterior, target)
