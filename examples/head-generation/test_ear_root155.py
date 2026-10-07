import unittest, json
from pathlib import Path
from test_style_math import pure_function

C = json.loads(Path(__file__).with_name("face_style.json").read_text())["calibration"]


class RootRelationshipTests(unittest.TestCase):
    def test_lower_attachment_leads_middle_without_moving_upper_root(self):
        curve, _ = pure_function("face.py", "curve", {})
        low = curve(0.28, 1, C["continuous_back_profile"])
        middle = curve(0.38, 1, C["continuous_back_profile"])
        self.assertGreater(low - middle, 0.02)
        self.assertLess(low - middle, 0.045)
        self.assertAlmostEqual(curve(0.52, 1, C["continuous_back_profile"]), -0.354)

    def test_back_return_leaves_real_open_separation(self):
        self.assertLess(C["ear_return_basin_depth_fraction"], 0.8)
        self.assertTrue(1.1 < C["ear_return_basin_width_fraction"] < 1.5)
