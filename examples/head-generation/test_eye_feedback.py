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
