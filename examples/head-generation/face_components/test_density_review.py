"""Density comparisons must use stable masks and fail closed outside their scope."""
from copy import deepcopy
import unittest
from unittest.mock import patch

import numpy as np
import density_review as review


def sample(height=1):
    return {
        "v": np.array([[-.5,0,0],[.5,0,0],[.5,height,0],[-.5,height,0]], dtype=float),
        "f": np.array([[0,1,2],[0,2,3]], dtype=int),
        "uv": np.full((4,2), .2), "sha256": "sample", "morphs": [],
    }


class DensityReviewTests(unittest.TestCase):
    def test_baseline_frame_is_reused_for_changed_height(self):
        with patch.object(review, "read", side_effect=[sample(), sample(2)]):
            result = review.compare("before", "after")
        self.assertEqual(result["baseline"]["normalization"], result["candidate"]["normalization"])
        self.assertEqual(result["candidate"]["normalization"]["height_m"], 1)

    def test_unrelated_extreme_does_not_move_skin_frame(self):
        data = sample()
        data["v"] = np.vstack((data["v"], [[0,10,0],[.01,10,0],[0,10.01,0]]))
        data["uv"] = np.vstack((data["uv"], np.full((3,2), .8)))
        data["f"] = np.vstack((data["f"], [[4,5,6]]))
        with patch.object(review, "read", return_value=data):
            result = review.analyze("sample")
        self.assertEqual(result["normalization"]["height_m"], 1)
        self.assertEqual(result["skin_triangles"], 2)

    def test_skin_selection_uses_chart_area_not_triangle_rank(self):
        data = sample()
        data["v"] = np.vstack((data["v"], [[0,10,0],[.01,10,0],[0,10.01,0]]))
        data["uv"] = np.full((7,2), .2)
        data["f"] = np.vstack((data["f"], np.tile([4,5,6], (8,1))))
        with patch.object(review, "read", return_value=data):
            result = review.analyze("sample")
        self.assertEqual(result["skin_triangles"], 2)

    def test_empty_skin_and_reversed_front_reject(self):
        for change in ("uv", "winding"):
            data = sample()
            if change == "uv":
                data["uv"][:] = .8
            else:
                data["f"] = data["f"][:, ::-1]
            with patch.object(review, "read", return_value=data), self.assertRaises(ValueError):
                review.analyze("sample")

    def test_empty_adjacency_rejects_instead_of_nan_percentiles(self):
        with self.assertRaises(ValueError):
            review.adjacent_ratios(np.array([[0,1,2]]), [0], np.arange(3), np.array([1.]))


if __name__ == "__main__":
    unittest.main()
