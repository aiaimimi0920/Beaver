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
