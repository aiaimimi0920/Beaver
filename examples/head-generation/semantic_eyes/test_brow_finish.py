"""Eyebrow-only finish controls preserve aperture, lashes and face calibration."""

import ast
import math
import copy
import json
from pathlib import Path
import unittest
from parameters import adapt_style, validate, adapt_brow_source


class BrowFinishTests(unittest.TestCase):
    def setUp(self):
        self.style = json.loads(
            (
                Path(__file__).parents[1] / "checkpoints/current/face_style.json"
            ).read_text()
        )

    def test_defaults_are_identity(self):
        self.assertEqual(adapt_style(self.style, {}), self.style)

    def test_only_expected_fields_change(self):
        for illustrated in (False, True):
            original = dict(self.style, illustrated_eye_art=illustrated)
            expected = adapt_style(original, {"eye_spacing": 0.99})
            result = adapt_style(
                original,
                dict(eye_spacing=0.99, brow_fullness_offset=0.20, brow_softness=0.4),
            )
            adjusted = copy.deepcopy(result)
            self.assertAlmostEqual(
                result["calibration"]["brow_inner_fullness"],
                expected["calibration"]["brow_inner_fullness"] + 0.20,
            )
            for old, new in zip(expected["brow_rgb"], result["brow_rgb"]):
                self.assertGreater(new, old)
                self.assertAlmostEqual(new, old + (1 - old) * 0.048)
            adjusted["calibration"]["brow_inner_fullness"] = expected["calibration"][
                "brow_inner_fullness"
            ]
            adjusted["brow_rgb"] = expected["brow_rgb"]
            self.assertEqual(adjusted, expected)

    def test_fullness_endpoints_keep_width_positive(self):
        for value in (0.0, 0.25):
            result = adapt_style(self.style, {"brow_fullness_offset": value})
            fullness = result["calibration"]["brow_inner_fullness"]
            for i in range(25):
                self.assertGreater(1 + fullness * (1 - 2 * i / 24), 0)

    def test_rounded_head_retains_centerline_and_closed_endpoints(self):
        source = (Path(__file__).parents[1] / "checkpoints/current/face.py").read_text()
        self.assertEqual(adapt_brow_source(source, 0), source)
        for amount in (0.25, 1.0):
            changed = adapt_brow_source(source, amount)
            compile(changed, "face.py", "exec")
            tree = ast.parse(changed)
            node = next(
                n
                for n in tree.body
                if isinstance(n, ast.FunctionDef) and n.name == "brow_lane_offset"
            )
            c = dict(self.style["calibration"], brow_head_rounding=amount)
            ns = {"C": c, "H": 1, "sin": math.sin, "pi": math.pi}
            exec(compile(ast.Module(body=[node], type_ignores=[]), "brow", "exec"), ns)
            fn = ns["brow_lane_offset"]
            for t in (0, 0.02, 0.1, 0.25, 0.5, 0.9, 1):
                a, b = fn(t, 0), fn(t, 1)
                original = c["brow_thickness"] * math.sin(math.pi * t) ** 0.7
                self.assertAlmostEqual((a + b) / 2, original / 2)
                self.assertGreaterEqual(b - a, 0)
                if t in (0, 1):
                    self.assertAlmostEqual(b - a, 0, places=8)
                else:
                    self.assertGreaterEqual(
                        b - a, original * (1 + c["brow_inner_fullness"] * (1 - 2 * t))
                    )
        with self.assertRaises(ValueError):
            adapt_brow_source("bad source", 1)

    def test_level_rise_endpoint_is_brow_only(self):
        before = adapt_style(self.style, {"eye_spacing": 0.99})
        after = adapt_style(
            self.style, {"eye_spacing": 0.99, "brow_rise_offset": -0.0255}
        )
        self.assertAlmostEqual(after["calibration"]["brow_rise"], 0)
        after["calibration"]["brow_rise"] = before["calibration"]["brow_rise"]
        self.assertEqual(after, before)

    def test_invalid_controls(self):
        for key, limit in [
            ("brow_fullness_offset", 0.25),
            ("brow_softness", 1),
            ("brow_head_rounding", 1),
        ]:
            for value in [-0.01, limit + 0.01, True, float("nan"), float("inf"), "0.1"]:
                with self.assertRaises(ValueError):
                    validate({key: value})
