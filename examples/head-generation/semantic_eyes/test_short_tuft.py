"""Explicit isolated short-leaf controls preserve neutral behavior."""

import json
from pathlib import Path
import unittest
from parameters import adapt_style, validate


class ShortTuftTests(unittest.TestCase):
    def setUp(self):
        self.style = json.loads(
            (
                Path(__file__).parents[1] / "checkpoints/current/face_style.json"
            ).read_text()
        )

    def test_neutral_and_incompatible_requests(self):
        self.assertEqual(adapt_style(self.style, {}), self.style)
        for request in (
            {"short_upper_tuft": 1},
            {"short_upper_tuft": 0.5},
            {
                "short_upper_tuft": 1,
                "supplemental_lashes": 0,
                "lash_contour_follow": 0.5,
            },
        ):
            with self.assertRaises(ValueError):
                validate(request)

    def test_direct_control_limits_depth_without_art_profile(self):
        p = {"short_upper_tuft": 1, "supplemental_lashes": 0, "lash_contour_follow": 1}
        result = adapt_style(self.style, p)
        expected = {
            "accent_tuft_flare_m": 0.0006,
            "accent_tuft_free_bow_m": 0.0003,
            "accent_tuft_rear_scoop_m": 0.0004,
            "accent_tuft_root_cover_m": 0.00015,
            "accent_tuft_mid_recede_m": 0.00015,
        }
        for key, value in expected.items():
            self.assertEqual(result[key], value)
        self.assertEqual(
            result["nose_plane_tint_strength"], self.style["nose_plane_tint_strength"]
        )

    def test_leaf_is_derived_from_adapted_parent(self):
        p = {"short_upper_tuft": 1, "supplemental_lashes": 0, "lash_contour_follow": 1}
        a = adapt_style(self.style, p)
        b = adapt_style(self.style, dict(p, upper_arch=1.2))
        self.assertNotEqual(
            a["upper_swept_tuft_outline"], b["upper_swept_tuft_outline"]
        )
        leaf = a["upper_swept_tuft_outline"]
        self.assertEqual(min(v[0] for v in leaf), 0.42)
        self.assertEqual(max(v[0] for v in leaf), 0.95)
        self.assertEqual(max(leaf, key=lambda v: v[0]), max(leaf, key=lambda v: v[1]))
        self.assertEqual(
            self.style["calibration"]["nose_stations"],
            a["calibration"]["nose_stations"],
        )


if __name__ == "__main__":
    unittest.main()
