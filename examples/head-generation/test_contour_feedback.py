import unittest
import math
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
        self.assertAlmostEqual(
            z, 0.04 + (0.078 - 0.236 * 0.25 - 0.04) * C["jaw_return_depth_strength"]
        )

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
            target = (
                C["ear_root_depth_base"]
                + C["ear_root_depth_slope"] * t
                - C["ear_posterior_span"]
                * math.sin(math.pi * t)
                * (1 + C["ear_upper_fullness"] * (2 * t - 1))
            )
            for root in [-0.30, -0.33, -0.36]:
                _, posterior = fn(t, 1, C, root)
                self.assertAlmostEqual(root - posterior, target)


class EarBasinTests(unittest.TestCase):
    def test_ear_basin_preserves_root_and_rim(self):
        curve, _ = pure_function("face.py", "curve", {})
        fn, _ = pure_function("ears.py", "membrane_offset", {"curve": curve})
        for t in [0.1, 0.3, 0.55, 0.78, 0.94]:
            self.assertEqual(fn(t, 0, C, -0.35), (0, 0))
            self.assertAlmostEqual(
                -0.35 - fn(t, 1, C, -0.35)[1],
                (
                    C["ear_root_depth_base"]
                    + C["ear_root_depth_slope"] * t
                    - C["ear_posterior_span"]
                    * math.sin(math.pi * t)
                    * (1 + C["ear_upper_fullness"] * (2 * t - 1))
                ),
            )

    def test_visible_ear_cross_section_does_not_flip(self):
        curve, _ = pure_function("face.py", "curve", {})
        fn, _ = pure_function("ears.py", "membrane_offset", {"curve": curve})
        points = [fn(0.55, u / 100, C, -0.35) for u in range(101)]
        self.assertTrue(all(b[0] > a[0] and b[1] > a[1] for a, b in zip(points, points[1:])))
        self.assertLess(points[51][1] - points[50][1], points[1][1] - points[0][1])

    def test_upper_ear_arc_rises_above_attachment(self):
        import math

        low, high = C["ear_root_min_height"], C["ear_root_max_height"]
        top = max(
            low
            + (high - low) * (1 - math.cos(math.pi * t / 100)) / 2
            + C["ear_upper_arc_bias"] * (t / 100) * math.sin(math.pi * t / 100)
            for t in range(101)
        )
        self.assertGreater(top, high + 0.01)
        self.assertLess(top, high + 0.035)

    def test_lower_ear_arc_has_horizontal_tangent(self):
        fn, _ = pure_function("ears.py", "ear_arc_height", {"H": 1, "C": C})
        low, high = C["ear_root_min_height"], C["ear_root_max_height"]
        self.assertLess(abs((fn(0.0001, low, high) - fn(0, low, high)) / 0.0001), 0.001)


class EarProjectionTests(unittest.TestCase):
    def test_projection_preserves_side_plane_spacing_and_mirroring(self):
        fn, _ = pure_function("ears.py", "project_ear_uv", {})
        points = [(1, 0, 0), (2, 0.2, 0.3), (3, 0.5, 0.7), (4, 1, 1)]
        result = fn(points)
        self.assertEqual(result, fn([(-x, y, z) for x, y, z in points]))
        self.assertEqual(result[0], (0.02, 0.02))
        self.assertEqual(result[-1], (0.98, 0.98))
        self.assertAlmostEqual(result[2][0] - result[1][0], 0.96 * 0.3)
        self.assertTrue(all(0.02 <= x <= 0.98 and 0.02 <= y <= 0.98 for x, y in result))


class EarAsymmetryTests(unittest.TestCase):
    def test_upper_body_is_fuller_than_lower_lobe(self):
        fn, _ = pure_function("ears.py", "membrane_offset", {})
        for low in [0.15, 0.25, 0.35]:
            self.assertGreater(fn(1 - low, 1, C, -0.35)[0], fn(low, 1, C, -0.35)[0])
        self.assertEqual(fn(0.5, 0, C, -0.35), (0, 0))
        self.assertGreater(C["ear_upper_fullness"], 0)
        self.assertLess(C["ear_upper_fullness"], 0.6)


class ChinViewTests(unittest.TestCase):
    def test_underside_arch_is_bounded_and_keeps_center(self):
        fn, _ = pure_function("shell.py", "chin_underside_height", {"C": C, "H": 1, "Y0": 0})
        self.assertEqual(fn(0.03, 0), 0.03)
        self.assertAlmostEqual(fn(0, 1), C["chin_return_lift"])
        heights = [fn(0.03, i / 20) for i in range(21)]
        self.assertTrue(all(b > a for a, b in zip(heights, heights[1:])))
        self.assertLess(heights[-1] - heights[0], 0.03)
        self.assertEqual(C["chin_return_lift"], 0.008)
        self.assertEqual(C["width_stations"][0], [0, 0.025])


class ChinPosteriorBowTests(unittest.TestCase):
    def test_bow_preserves_front_and_side_attachments(self):
        fn, _ = pure_function("shell.py", "chin_underside_depth_offset", {"C": C, "H": 1})
        self.assertEqual(fn(0.03, 0, 0), 0.03)
        self.assertEqual(fn(0.2, 0.03, 1), 0.2)
        self.assertAlmostEqual(fn(0.2, 0.03, 0) - 0.2, C["chin_underside_center_back"])
        self.assertLess(fn(0.2, 0.03, 0) - 0.2, 0.02)


class ChinRoundnessTests(unittest.TestCase):
    def test_rounded_bow_has_smooth_center_and_monotone_depth(self):
        fn, _ = pure_function("shell.py", "chin_underside_depth_offset", {"C": C, "H": 1})
        values = [fn(0.2, 0.03, i / 20) for i in range(21)]
        self.assertTrue(all(b < a for a, b in zip(values, values[1:])))
        self.assertLess(abs((fn(0.2, 0.03, 0.0001) - fn(0.2, 0.03, 0)) / 0.0001), 0.001)
        self.assertEqual(values[-1], 0.2)


class WrappedPinnaTests(unittest.TestCase):
    def test_recess_keeps_root_and_outer_outline_but_changes_basin(self):
        fn, _ = pure_function("ears.py", "membrane_offset", {})
        flat = dict(C, ear_cup_recess=0)
        for u in [0, 1]:
            self.assertEqual(fn(.5, u, C, -.34), fn(.5, u, flat, -.34))
        self.assertLess(fn(.5, .5, C, -.34)[0], fn(.5, .5, flat, -.34)[0])

    def test_no_duplicate_back_membrane_in_recipe(self):
        source = Path(__file__).with_name("ears.py").read_text()
        self.assertNotIn("vs.extend((x - side * thickness", source)
        self.assertNotIn("for i in reversed(f)) for f in front_faces", source)
        self.assertGreater(C["ear_return_lip_width"], .04)
        self.assertLess(C["ear_return_lip_width"], C["ear_outward_span"])
