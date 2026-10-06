import json
import unittest
from pathlib import Path

C = json.loads(Path(__file__).with_name("face_style.json").read_text())["calibration"]


class EyeFeedbackTests(unittest.TestCase):
    def test_rear_depth_is_shorter_but_behind_iris(self):
        front = min(C["eye_inner_depth"], C["eye_outer_depth"]) - C["pocket_transition_back"]
        back = front - C["pocket_body_length"]
        self.assertGreater(back, -0.12)
        self.assertLess(front, C["iris_rim_depth"] - C["iris_concavity"])
        self.assertLess(C["pocket_body_length"], 0.065)

    def test_catchlight_stays_inside_iris_parameter_domain(self):
        for x in [-1, 1]:
            for y in [-1, 1]:
                dx = C["catchlight_offset_x"] + x * C["catchlight_half_width"]
                dy = C["catchlight_offset_y"] + y * C["catchlight_half_height"]
                self.assertLess(
                    (dx / C["iris_half_width"]) ** 2 + (dy / C["iris_half_height"]) ** 2, 0.7
                )
        self.assertLess(C["catchlight_surface_offset"], 0.002)
        self.assertEqual(C["catchlight_grid_resolution"], 5)


class LashPlaneTests(unittest.TestCase):
    def test_ink_stays_just_in_front_of_local_rim(self):
        import json
        from pathlib import Path
        from test_style_math import pure_function

        c = json.loads(Path(__file__).with_name("face_style.json").read_text())["calibration"]
        rim, _ = pure_function(
            "face.py", "rim_depth", {"C": c, "H": c["height_m"], "Z0": c["chin_depth_m"]}
        )
        fn, _ = pure_function("face.py", "lash_depth", {"rim_depth": rim})
        for x in [0.03, 0.05, 0.07]:
            self.assertAlmostEqual(fn(x, 1.65) - rim(x, 1.65), 0.0015)
            self.assertEqual(fn(x, 1.65), fn(x, 1.64))


class BrowTaperTests(unittest.TestCase):
    def test_inner_body_fuller_outer_tail_and_center_preserved(self):
        import json, math
        from pathlib import Path
        from test_style_math import pure_function

        c = json.loads(Path(__file__).with_name("face_style.json").read_text())["calibration"]
        fn, _ = pure_function("face.py", "brow_lane_offset", {"C": c, "H": 1})
        self.assertGreater(fn(0.1, 1) - fn(0.1, 0), fn(0.9, 1) - fn(0.9, 0))
        for t in [0.1, 0.3, 0.5, 0.7, 0.9]:
            self.assertAlmostEqual(
                (fn(t, 0) + fn(t, 1)) / 2, 0.5 * c["brow_thickness"] * math.sin(math.pi * t) ** 0.7
            )
        self.assertEqual(fn(0, 0), fn(0, 1))


class RoundedApertureTests(unittest.TestCase):
    def test_rounding_preserves_corners_center_and_symmetry(self):
        import json
        from pathlib import Path
        from test_style_math import pure_function

        c = json.loads(Path(__file__).with_name("face_style.json").read_text())["calibration"]
        fn, _ = pure_function("face.py", "eye_contour", {"C": c, "H": 1, "Y0": 0})
        for t in [-1, 0, 1]:
            a, b = fn(1, t, True), fn(-1, t, True)
            self.assertAlmostEqual(a[0], -b[0])
            self.assertAlmostEqual(a[1], b[1])
        for t in [-1, 1]:
            self.assertEqual(fn(1, t, True), fn(1, t, False))
        for t in [-0.95, -0.8, -0.4, 0, 0.4, 0.8, 0.95]:
            self.assertGreater(fn(1, t, True)[1], fn(1, t, False)[1])
        old = dict(c, eye_horizontal_power=2)
        classic, _ = pure_function("face.py", "eye_contour", {"C": old, "H": 1, "Y0": 0})
        self.assertGreater(
            fn(1, 0.8, True)[1] - fn(1, 0.8, False)[1],
            classic(1, 0.8, True)[1] - classic(1, 0.8, False)[1],
        )


class OrbitalInfluenceTests(unittest.TestCase):
    def test_far_lateral_face_is_not_pulled_toward_eye(self):
        import json
        from pathlib import Path
        from test_style_math import pure_function

        c = json.loads(Path(__file__).with_name("face_style.json").read_text())["calibration"]
        fn, _ = pure_function(
            "face.py",
            "depth",
            {"C": c, "H": 1, "Y0": 0, "base_depth": lambda x, y: -1, "rim_depth": lambda x, y: 1},
        )
        for t in [1.17, 1.35, 1.6]:
            self.assertAlmostEqual(
                fn(c["eye_center_x"] + c["eye_half_width"] * t, c["eye_corner_y"]), -1
            )
        self.assertGreater(fn(c["eye_center_x"], c["eye_corner_y"] + c["eye_upper_arch"]), 0.99)
