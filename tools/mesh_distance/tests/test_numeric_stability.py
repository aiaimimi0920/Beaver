"""Near-feature accuracy with distant vertices, including admitted-domain failures."""
import itertools
import unittest

import numpy as np
from oracle_decimal import decimal_triangle
from test_support import mesh


class NumericStabilityTests(unittest.TestCase):
    def check_small_feature(self, triangle, point):
        # V5 explicitly does not support the original high-scale regressions.
        # Keep every such witness as an unsupported test, not a repaired claim.
        if abs(triangle).max() > 16 or abs(point).max() > 16:
            for permutation in itertools.permutations(range(3)):
                with self.assertRaisesRegex(ValueError, "absolute coordinate cap"):
                    mesh(triangle[list(permutation)], [[0, 1, 2]])
            return False
        expected, hit = decimal_triangle(triangle, point)
        # Deliberately NOT scaled by the huge remote vertex: that would hide
        # a metre-scale failure around a sub-metre local feature.
        tolerance = 32 * np.finfo(float).eps * max(1., np.linalg.norm(point), np.linalg.norm(hit))
        for permutation in itertools.permutations(range(3)):
            with mesh(triangle[list(permutation)], [[0, 1, 2]]) as index:
                distance, _, actual_hit = index.query([point], True)
            self.assertLessEqual(abs(distance[0] - expected), tolerance)
            self.assertLessEqual(np.linalg.norm(actual_hit[0] - hit), tolerance)
        return True

    def test_admitted_domain_distant_origin_witness(self):
        triangle = np.array([[-4.593059880741928e17, -3.292163831515091e17,
                              -7.178674706685751e17], [0., 0., 0.],
                             [.7451281500980258, -.4089402938261628, -.7648885417729616]])
        self.check_small_feature(triangle, np.array([.5, .25, .125]))

    def test_distant_origin_synthetic_permutations(self):
        rng = np.random.default_rng(92815)
        accepted = 0
        for exponent in (30, 40, 50, 55, 59):
            for _ in range(20):
                remote = rng.normal(size=3) * 2.**exponent
                local = np.round(rng.normal(size=3) * 2.**30) / 2.**30
                triangle = np.array([remote, [0., 0., 0.], local])
                accepted += self.check_small_feature(triangle, np.array([.5, .25, .125]))
        self.assertEqual(accepted, 0)  # All 100 original large inputs are unsupported.

    def test_balanced_large_triangle_preserves_small_query(self):
        for exponent in (30, 40, 50, 55, 59):
            length = 2.**exponent
            triangle = np.array([[length, length, 0.], [0., 0., 0.], [length, 0., 0.]])
            self.check_small_feature(triangle, np.array([.5, .25, .001]))

    def test_fast_path_remote_edge_endpoint(self):
        for exponent in (30, 40, 50, 52):
            triangle = np.array([[1., 1., 0.], [2.**exponent, 1., 0.],
                                 [1., 2.**exponent, 0.]])
            self.check_small_feature(triangle, np.array([.999, 1.3, .001]))

    def test_all_remote_skew_interior_reconstruction(self):
        for exponent in (0, 1, 2, 30, 35, 40, 45, 50, 55, 59):
            triangle = np.array([[-1., -2., 0.], [3., -1., 0.],
                                 [-2., 4., 0.]]) * 2.**exponent
            self.check_small_feature(triangle, np.array([.5, .25, .001]))

    def test_exact_residual_rejects_false_zero_area(self):
        # Exact cross_z=-2^-152, but rounded binary128 cross is zero.
        triangle = np.array([[2.**-100, 2.**-100, 0.],
                             [1., 1. + 2.**-52, 0.], [2., 2. + 2.**-51, 0.]])
        for permutation in itertools.permutations(range(3)):
            with self.assertRaisesRegex(ValueError, "Unresolved nonzero triangle area"):
                mesh(triangle[list(permutation)], [[0, 1, 2]])


if __name__ == "__main__":
    unittest.main()
