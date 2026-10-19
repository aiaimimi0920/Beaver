"""An isolated secondary blade can shrink without moving the eye or parent."""

import json
from pathlib import Path
import unittest
from parameters import adapt_style, validate


class SecondaryLashTests(unittest.TestCase):
    def test_parent_relative_scale_preserves_other_fields_and_root(self):
        base = json.loads(
            (
                Path(__file__).parents[1] / "checkpoints/current/face_style.json"
            ).read_text()
        )
        request = dict(short_upper_tuft=1, supplemental_lashes=0, lash_contour_follow=1)
        for illustrated in (False, True):
            original = dict(base, illustrated_eye_art=illustrated)
            before = adapt_style(original, request)
            self.assertEqual(
                before, adapt_style(original, dict(request, secondary_lash_scale=1))
            )
            rails = before["upper_lash_rails"]

            def mid(t):
                for a, b in zip(rails, rails[1:]):
                    if a[0] <= t <= b[0]:
                        q = (t - a[0]) / (b[0] - a[0])
                        return ((a[1] + a[2]) * (1 - q) + (b[1] + b[2]) * q) / 2
                raise AssertionError("Child escaped parent span")

            leaf = before["upper_swept_tuft_outline"]
            root = leaf[0][0]
            for scale in (0.70, 0.78, 0.85):
                after = adapt_style(original, dict(request, secondary_lash_scale=scale))
                for key in before:
                    if key != "upper_swept_tuft_outline":
                        self.assertEqual(before[key], after[key], key)
                changed = after["upper_swept_tuft_outline"]
                self.assertEqual(len(leaf), len(changed))
                self.assertEqual(leaf[0], changed[0])
                for (t, y), (nt, ny) in zip(leaf, changed):
                    self.assertAlmostEqual(nt, root + (t - root) * scale)
                    self.assertAlmostEqual(ny - mid(nt), (y - mid(t)) * scale)

    def test_invalid_dependency_and_range(self):
        valid = dict(short_upper_tuft=1, supplemental_lashes=0, lash_contour_follow=1)
        for value in (0.69, 1.01, True, float("nan"), float("inf"), "0.8"):
            with self.assertRaises(ValueError):
                validate(dict(valid, secondary_lash_scale=value))
        with self.assertRaises(ValueError):
            validate(dict(secondary_lash_scale=0.78))
