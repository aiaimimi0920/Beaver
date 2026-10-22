import copy
import json
from pathlib import Path
import unittest
from mouth_design import validate, apply
from mouth_motion import motion_delta, TARGETS, validate_weights

HERE = Path(__file__).resolve().parent
PRESETS = json.loads((HERE / "presets.json").read_text())
STYLE = json.loads((HERE.parent / "checkpoints/current/face_style.json").read_text())


class MouthDesignTests(unittest.TestCase):
    def test_all_presets_and_no_mutation(self):
        old = copy.deepcopy(STYLE)
        for value in PRESETS.values():
            self.assertEqual(validate(value), value)
            result = apply(STYLE, value)
            self.assertEqual(result["calibration"]["mouth_design"], value)
        self.assertEqual(old, STYLE)

    def test_invalid_domains_and_identity(self):
        for key, value in [
            ("half_width", True),
            ("lip_strength", float("nan")),
            ("muzzle_projection", 1.0),
            ("version", True),
            ("kind", "phoneme"),
        ]:
            design = {**PRESETS["line"], key: value}
            with self.assertRaises(ValueError):
                validate(design)
        with self.assertRaises(ValueError):
            validate({**PRESETS["line"], "unknown": 1})
        with self.assertRaises(ValueError):
            validate({**PRESETS["line"], "muzzle_projection": 0.01})

    def test_motion_symmetry_and_compact_support(self):
        c = STYLE["calibration"]
        center = c["origin_y_m"] + c["height_m"] * c["mouth_height"] - 0.021
        for target in TARGETS:
            a = motion_delta((0.01, -0.08, center), c, target)
            b = motion_delta((-0.01, -0.08, center), c, target)
            self.assertAlmostEqual(a[0], -b[0])
            self.assertEqual(a[1:], b[1:])
            self.assertEqual(
                motion_delta((0.1, -0.08, center), c, target), (0.0, 0.0, 0.0)
            )
            self.assertEqual(
                motion_delta((0.01, -0.08, center + 0.04), c, target), (0.0, 0.0, 0.0)
            )

    def test_channels_have_distinct_roles(self):
        c = STYLE["calibration"]
        z = c["origin_y_m"] + c["height_m"] * c["mouth_height"] - 0.021
        q = (0.01, -0.08, z)
        self.assertLess(motion_delta(q, c, "MouthRound")[0], 0)
        self.assertGreater(motion_delta(q, c, "MouthSpread")[0], 0)
        self.assertLess(
            motion_delta(q, c, "MouthPucker")[1], motion_delta(q, c, "MouthRound")[1]
        )

    def test_weight_rejection(self):
        for value in [True, -0.1, 1.1, float("nan"), "1"]:
            with self.assertRaises(ValueError):
                validate_weights({"MouthOpen": value})
        with self.assertRaises(ValueError):
            validate_weights({"aa": 1})
        self.assertEqual(validate_weights({})["MouthOpen"], 0.0)

    def test_support_field_bound(self):
        scope = {
            "C": {"mouth_design": PRESETS["round-muzzle"]},
            "H": 0.236,
            "Y0": 1.544,
        }
        exec((HERE / "mouth_field.py").read_text(), scope)
        fn = scope["researched_mouth_support"]
        bound = 0.236 * 0.035
        for x in [-0.05, -0.02, 0, 0.02, 0.05]:
            for h in [0, 0.04, 0.10, 0.14, 0.20, 0.24, 0.3]:
                v = fn(x, 1.544 + 0.236 * h)
                self.assertTrue(0 <= v <= bound + 1e-14)
                if abs(x) >= 0.236 * 0.16 or h <= 0.04 or h >= 0.24:
                    self.assertAlmostEqual(v, 0.0)

    def test_three_motion_fields_have_rank_three(self):
        import numpy as np

        c = STYLE["calibration"]
        z = c["origin_y_m"] + c["height_m"] * c["mouth_height"] - 0.021
        rows = []
        for x in (0.004, 0.012, 0.024):
            rows.extend(zip(*(motion_delta((x, -0.08, z), c, t) for t in TARGETS)))
        self.assertEqual(np.linalg.matrix_rank(np.array(rows)), 3)

    def test_muzzle_keeps_hidden_bag_behind_original_surface(self):
        import sys

        old = sys.path[:]
        sys.path.insert(0, str(HERE.parent / "facial_surface"))
        try:
            from source_mouth_templates import adapt_mouth_companions
            from mouth_source import adapt_companion

            source = (HERE.parent / "mouth.py").read_text()
            common = adapt_companion("mouth.py", source, PRESETS["line"])
            self.assertEqual(common, adapt_mouth_companions("mouth.py", source, True))
            muzzle = adapt_companion("mouth.py", source, PRESETS["round-muzzle"])
            self.assertIn(
                "+ researched_mouth_support(0, MOUTH_Y)) if ring >= 2", muzzle
            )
            self.assertIn(
                "depth(0, MOUTH_Y) - researched_mouth_support(0, MOUTH_Y) - 0.023",
                muzzle,
            )
            self.assertIn("assert max(signed_distances) < 0", muzzle)
            compile(muzzle, "muzzle companion", "exec")
        finally:
            sys.path[:] = old
