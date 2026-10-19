"""Independent outer-wing lift leaves the accepted aperture and root unchanged."""

import json
from pathlib import Path
import unittest
from bundle import RECIPE
from parameters import adapt_style, validate


class WingLiftTests(unittest.TestCase):
    def setUp(self):
        self.style = json.loads((RECIPE / "face_style.json").read_text())
        self.p = json.loads(
            (
                Path(__file__).parents[1] / "eye_presets" / "soft-downturned-798.json"
            ).read_text()
        )["eye_parameters"]

    def test_default_identity_dependency_and_bounds(self):
        self.assertEqual(adapt_style(self.style, {"wing_lift_scale": 1}), self.style)
        for value in (-0.01, 1.01, True, float("nan")):
            with self.assertRaises(ValueError):
                validate(dict(self.p, wing_lift_scale=value))
        with self.assertRaises(ValueError):
            validate({"wing_lift_scale": 0.5})

    def test_only_wing_moves_root_preserved_and_width_unchanged(self):
        before = adapt_style(self.style, self.p)
        after = adapt_style(self.style, dict(self.p, wing_lift_scale=0.5))
        changed = {k for k in after if after.get(k) != before.get(k)}
        self.assertEqual(changed, {"outer_lash_wing_rails", "accent_wing_lift_scale"})
        a, b = before["outer_lash_wing_rails"], after["outer_lash_wing_rails"]
        self.assertEqual(len(a), len(b))
        self.assertEqual(a[0], b[0])
        start, end = a[0][0], a[-1][0]
        for old, new in zip(a, b):
            u = (old[0] - start) / (end - start)
            self.assertEqual(old[0], new[0])
            self.assertAlmostEqual(new[2] - new[1], old[2] - old[1])
            for lane in (1, 2):
                self.assertAlmostEqual(
                    new[lane] - old[lane], -0.011 * u * u * (3 - 2 * u)
                )

    def test_native_tip_uses_same_scale(self):
        # The generation source carries the same bounded request through STYLE.
        source = (Path(__file__).parent / "bundle.py").read_text()
        self.assertIn('H * 0.012 * STYLE.get("accent_wing_lift_scale", 1.0)', source)
        self.assertIn("if candidate.length < .004:", source)
