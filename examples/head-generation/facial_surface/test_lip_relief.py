"""Upper-lip experiment is opt-in, bounded, and leaves other controls intact."""
import copy
import json
import math
from pathlib import Path
import unittest
from source_lips import apply_lip_relief


class LipReliefTests(unittest.TestCase):
    def setUp(self):
        self.style = json.loads((Path(__file__).parents[1] /
            "checkpoints/current/face_style.json").read_text())

    def test_identity_and_no_mutation(self):
        original = copy.deepcopy(self.style)
        for p in (None, {}, {"upper_lip_relief_scale": 1}):
            result, report = apply_lip_relief(self.style, p)
            self.assertEqual(result, original)
            self.assertIsNone(report)
        self.assertEqual(self.style, original)

    def test_only_relief_changes_and_bound_covers_formula(self):
        result, report = apply_lip_relief(self.style, {"upper_lip_relief_scale": .9})
        c = self.style["calibration"]
        self.assertAlmostEqual(result["calibration"]["upper_lip_relief"], .9*c["upper_lip_relief"])
        result["calibration"]["upper_lip_relief"] = c["upper_lip_relief"]
        self.assertEqual(result, self.style)
        for x in (-.1, -.02, 0, .02, .1):
            for dy in (-.03, -.01, 0, .01, .03):
                weight = math.exp(-(x/(c["height_m"]*c["upper_lip_lateral_width"]))**c["upper_lip_lateral_power"])
                weight *= math.exp(-(dy/(c["height_m"]*c["upper_lip_width"]))**2)
                delta = c["height_m"]*c["upper_lip_relief"]*.1*weight
                self.assertLessEqual(delta, report["analytic_depth_change_bound_m"])

    def test_invalid_controls(self):
        for p in ([], {"other":1}, {"upper_lip_relief_scale":True},
                  {"upper_lip_relief_scale":float('nan')},
                  {"upper_lip_relief_scale":.849}, {"upper_lip_relief_scale":1.01}):
            with self.assertRaises(ValueError):
                apply_lip_relief(self.style, p)

    def test_nonfinite_or_unbounded_authored_calibration(self):
        for key, value in (("height_m", float("nan")),
                           ("upper_lip_relief", float("inf")),
                           ("upper_lip_lateral_power", 3),
                           ("upper_lip_lateral_width", 0)):
            style = copy.deepcopy(self.style)
            style["calibration"][key] = value
            with self.assertRaises(ValueError):
                apply_lip_relief(style, {"upper_lip_relief_scale": .9})

    def test_lateral_falloff_bound_and_scope(self):
        result, report = apply_lip_relief(self.style, {"upper_lip_lateral_power": 2})
        c = self.style["calibration"]
        self.assertEqual(result["calibration"]["upper_lip_lateral_power"], 2)
        result["calibration"]["upper_lip_lateral_power"] = c["upper_lip_lateral_power"]
        self.assertEqual(result, self.style)
        for i in range(-20000, 20001):
            u = i / 1000
            delta = c["height_m"] * c["upper_lip_relief"] * abs(math.exp(-u*u) - math.exp(-u**4))
            self.assertLessEqual(delta, report["analytic_depth_change_bound_m"])
        for power in (True, 0, 3, float('nan'), '2'):
            with self.assertRaises(ValueError):
                apply_lip_relief(self.style, {"upper_lip_lateral_power":power})
