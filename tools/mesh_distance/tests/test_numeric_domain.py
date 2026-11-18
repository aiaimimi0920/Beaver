"""Fail closed on unsupported coordinates/conditioning instead of returning bad hits."""
import itertools
import unittest

import numpy as np
from oracle_decimal import decimal_triangle
from test_support import mesh


class NumericDomainTests(unittest.TestCase):
    def test_review_sliver_is_rejected_not_silently_a_line(self):
        triangle = np.array([[1e40, 1e40, 0.], [0., 0., 0.], [1., 0., 0.]])
        point = np.array([.5, .25, .001])
        oracle_distance, oracle_hit = decimal_triangle(triangle, point)
        self.assertEqual(oracle_distance, .001)
        np.testing.assert_array_equal(oracle_hit, [.5, .25, 0.])
        # Original failure returned .250002 and a wrong segment witness.
        for scale in (1e-40, 1., 1e40):
            for permutation in itertools.permutations(range(3)):
                with self.assertRaisesRegex(ValueError, "numerical domain"):
                    mesh(triangle[list(permutation)] * scale, [[0, 1, 2]])

    def test_nonzero_area_below_domain_is_not_accepted_as_degenerate(self):
        triangle = np.array([[0., 0., 0.], [1., 0., 0.], [.5, 2.**-61, 0.]])
        for permutation in itertools.permutations(range(3)):
            with self.assertRaisesRegex(ValueError, "conditioning"):
                mesh(triangle[list(permutation)], [[0, 1, 2]])

    def test_coordinate_span_limit_and_per_query_batch_independence(self):
        triangle = np.array([[0., 0., 0.], [1., 0., 0.], [0., 1., 0.]])
        with mesh(triangle, [[0, 1, 2]]) as index:
            self.assertTrue(np.isfinite(index.query([[2.**-111, .25, .001]])).all())
            with self.assertRaisesRegex(ValueError, "Query significant-bit span"):
                index.query([[2.**-112, .25, .001]])
        # Separate query points are not compared to one another: batching must
        # not expand the required coordinate domain in a zero-only target axis.
        with mesh(np.zeros((3, 3)), [[0, 1, 2]]) as index:
            points = np.array([[1e-100, 0., 0.], [16., 0., 0.]])
            together = index.query(points)
            separate = np.array([index.query([point])[0] for point in points])
            np.testing.assert_array_equal(together, separate)

    def test_significant_bit_boundary_and_subnormal_values(self):
        # Actual significant-bit span, not an unconditionally 53-bit mantissa.
        triangle = np.array([[0., 0., 0.], [2.**4, 0., 0.],
                             [2.**-55 + 2.**-107, 1., 0.]])
        with mesh(triangle, [[0, 1, 2]]) as index:
            self.assertEqual(index.query([[1., .25, .001]])[0], .001)
        triangle[2, 0] = 2.**-56 + 2.**-108
        with self.assertRaisesRegex(ValueError, "significant-bit span"):
            mesh(triangle, [[0, 1, 2]])
        tiny = np.nextafter(0., 1.)
        triangle = np.array([[0., 0., 0.], [tiny, 0., 0.], [0., tiny, 0.]])
        with mesh(triangle, [[0, 1, 2]]) as index:
            distance = index.query([[0., 0., tiny]])
            self.assertEqual(distance[0], tiny)

    def test_genuine_rotated_collinear_triangle_remains_supported(self):
        triangle = np.array([[0., 0., 0.], [1., 2., 3.], [2., 4., 6.]])
        points = np.array([[.5, .25, .001], [3., 6., 9.], [0., 0., 0.]])
        for permutation in itertools.permutations(range(3)):
            with mesh(triangle[list(permutation)], [[0, 1, 2]]) as index:
                distance = index.query(points)
            oracle = np.array([decimal_triangle(triangle, point)[0] for point in points])
            np.testing.assert_allclose(distance, oracle, rtol=1e-15, atol=0)


if __name__ == "__main__":
    unittest.main()
