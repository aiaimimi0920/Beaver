"""Regression checks for explicit author shaping and exact sampled validation."""
import unittest
import numpy as np
from forehead_exact_distance import forehead_exact_distance
from chin_source_shape import chin_shape_exact_distance
from forehead_author_pose_guard import guard_forehead_author_poses
from forehead_flow_acceptance import measure_forehead_flow
from forehead_retile_shape import guard_forehead_retile_poses


class CurrentForeheadTests(unittest.TestCase):
    def plane(self):
        return np.array([[0., 0., 0.], [.01, 0., 0.], [.01, .01, 0.], [0., .01, 0.]], np.float32)

    def test_conservative_triangle_pruning_matches_exhaustive(self):
        rng = np.random.default_rng(244)
        vertices = rng.normal(size=(90, 3))
        triangles = np.arange(90).reshape(-1, 3)
        points = np.r_[rng.normal(size=(50, 3)), vertices[:8], [[100., 0., 0.]]]
        exact = chin_shape_exact_distance(points, vertices, triangles)
        pruned = forehead_exact_distance(points, vertices, triangles)
        np.testing.assert_allclose(pruned, exact, rtol=1e-12, atol=1e-12)

    def test_degenerate_or_nonfinite_distance_geometry_rejected(self):
        for vertices in (np.zeros((3, 3)), np.full((3, 3), np.nan)):
            with self.assertRaises(ValueError):
                forehead_exact_distance([[0., 0., 0.]], vertices, [[0, 1, 2]])

    def test_author_shape_has_separate_correspondence_budget(self):
        old = self.plane()
        new = old.copy()
        new[:, 2] = .0015
        proof = guard_forehead_author_poses(old, new, [[0, 1, 2, 3]], range(4),
                                           np.zeros_like(old), lambda p: 0., lambda p, n: np.zeros(3))
        self.assertEqual(proof['states'], 729)
        self.assertAlmostEqual(proof['maximum_corresponding_vertex_displacement_mm'], 1.5, places=5)
        new[:, 2] = .0021
        with self.assertRaises(ValueError):
            guard_forehead_author_poses(old, new, [[0, 1, 2, 3]], range(4),
                                       np.zeros_like(old), lambda p: 0., lambda p, n: np.zeros(3))

    def test_identical_pose_cache_preserves_all_named_cases(self):
        old = self.plane()
        proof = guard_forehead_retile_poses(old, [[0, 1, 2, 3]], old, [[0, 1, 2, 3]],
            [0], [0], np.zeros_like(old), np.zeros_like(old), lambda *args: {0: 1.},
            lambda p: 0., lambda p, n: np.zeros(3))
        self.assertEqual(len(proof['cases']), 11)
        self.assertEqual(proof['unique_geometry_queries'], 1)
        self.assertEqual(proof['orientation_states'], 729)
        self.assertEqual(sum('reused_from_pose' in p for p in proof['cases']), 10)

    def test_dense_flow_measurement_includes_stations(self):
        t = np.linspace(0., 1., 20)
        points = np.column_stack((.1*t, .005*t*t, 1.7-.03*t*t))
        proof = measure_forehead_flow(points, list(range(len(points))))
        self.assertGreaterEqual(proof['sampled_centers'], 20001)
        self.assertLess(proof['upper_max_adjacent_length_ratio'], 1.1)
        self.assertTrue(np.isfinite(proof['sampled_peak_degrees']))


if __name__ == '__main__':
    unittest.main()
