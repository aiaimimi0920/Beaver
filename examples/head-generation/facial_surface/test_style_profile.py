"""Opt-in art direction is explicit, bounded by the existing packaging gate."""

import copy
import json
from pathlib import Path
import unittest
from bundle import apply_style_profile


class StyleProfileTests(unittest.TestCase):
    def setUp(self):
        self.base = json.loads(
            (
                Path(__file__).parents[1] / "checkpoints/current/face_style.json"
            ).read_text()
        )

    def test_identity_and_no_mutation(self):
        before = copy.deepcopy(self.base)
        neutral = apply_style_profile(self.base, None)
        self.assertEqual(neutral, before)
        self.assertIsNot(neutral, self.base)
        clean = apply_style_profile(self.base, "clean_emerald")
        self.assertEqual(self.base, before)
        self.assertIsNot(clean["calibration"], self.base["calibration"])
        for bad in ("", "unknown", True, 1, {}):
            with self.assertRaises(ValueError):
                apply_style_profile(self.base, bad)

    def test_eye_style_changes_and_preserved_anatomy(self):
        clean = apply_style_profile(self.base, "clean_emerald")
        old, new = self.base["calibration"], clean["calibration"]
        changed = {k for k in new if new[k] != old[k]}
        self.assertEqual(
            changed,
            {
                "iris_rim_depth",
                "iris_concavity",
                "pupil_half_width",
                "pupil_half_height",
            },
        )
        self.assertAlmostEqual(new["iris_rim_depth"] - old["iris_rim_depth"], -0.008)
        self.assertEqual(new["iris_concavity"], 0.010)
        self.assertEqual(new["nose_stations"], old["nose_stations"])
        self.assertEqual(
            new["continuous_lateral_power"], old["continuous_lateral_power"]
        )
        self.assertEqual(clean["lower_lash_width_scale"], 0.4)
        self.assertEqual(clean["accent_cross_section_profile"], "rounded")
        self.assertEqual(clean["accent_cross_arch_ratio"], 0.0)
        self.assertEqual(clean["brow_arch_m"], 0.0018)


if __name__ == "__main__":
    unittest.main()
