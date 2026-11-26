"""Metre-coordinate capability boundary, not a model-quality or threshold gate."""
from fractions import Fraction
import itertools
import unittest

import numpy as np
from oracle_decimal import decimal_triangle
from test_support import mesh


class BoundedDomainTests(unittest.TestCase):
    def compare_permutations(self, triangle, queries):
        reference = [decimal_triangle(triangle, p) for p in queries]
        distances = np.array([x[0] for x in reference])
        hits = np.array([x[1] for x in reference])
        for permutation in itertools.permutations(range(3)):
            with mesh(triangle[list(permutation)], [[0, 1, 2]]) as target:
                distance, _, hit = target.query(queries, True)
            self.assertLessEqual(abs(distance - distances).max(), 2e-14)
            self.assertLessEqual(abs(hit - hits).max(), 2e-14)
        # This numerical comparison is NOT an epsilon on the 1 mm gate.

    def test_absolute_boundary_targets_and_queries(self):
        for axis in range(3):
            for sign in (-1., 1.):
                vertex = np.zeros(3)
                vertex[axis] = sign * 16
                with mesh(np.tile(vertex, (3, 1)), [[0, 1, 2]]) as target:
                    np.testing.assert_array_equal(target.query([vertex]), [0.])
                outside = vertex.copy()
                outside[axis] = sign * np.nextafter(16., np.inf)
                with self.assertRaisesRegex(ValueError, "absolute coordinate cap"):
                    mesh(np.tile(outside, (3, 1)), [[0, 1, 2]])
                with mesh(np.zeros((3, 3)), [[0, 1, 2]]) as target:
                    self.assertEqual(target.query([vertex])[0], 16.)
                    with self.assertRaisesRegex(ValueError, "absolute coordinate cap"):
                        target.query([vertex, outside])
                    self.assertEqual(target.query([vertex])[0], 16.)

    def test_boundary_enclosing_interior_and_exact_threshold(self):
        triangle = np.array([[-16., -16., 0.], [16., -16., 0.], [0., 16., 0.]])
        heights = [0., np.nextafter(.001, 0), .001, np.nextafter(.001, np.inf)]
        queries = np.array([[0., 0., h] for h in heights])
        self.compare_permutations(triangle, queries)
        # Axis rotations/signs preserve the exact represented plane and heights.
        for axes in itertools.permutations(range(3)):
            for sign in (-1., 1.):
                with mesh(triangle[:, axes] * sign, [[0, 1, 2]]) as target:
                    result = target.query(queries[:, axes] * sign)
                np.testing.assert_array_equal(result, heights)
                np.testing.assert_array_equal(result > .001, [False, False, False, True])

    def test_exact_centroid_and_rotated_all_remote_triangles(self):
        rng = np.random.default_rng(2026110205)
        for _ in range(12):
            integers = rng.integers(-2**49, 2**49, size=(2, 3), dtype=np.int64)
            triangle = np.vstack([integers, -integers.sum(0)]).astype(float) * 2.**-48
            for axis in range(3):
                self.assertEqual(sum(Fraction(float(x)) for x in triangle[:, axis]), 0)
            queries = np.array([[0., 0., 0.], [.001, -.001, .001]])
            self.compare_permutations(triangle, queries)
            rotation, _ = np.linalg.qr(rng.normal(size=(3, 3)))
            rotated = triangle @ rotation
            self.assertLessEqual(abs(rotated).max(), 16.)
            # Rounded rotation need not preserve exact zero centroid; Decimal
            # measures the represented triangle rather than assuming it does.
            self.compare_permutations(rotated, queries @ rotation)

    def test_condition_boundary_without_geometric_tolerance(self):
        for height in (np.nextafter(2.**-60, 0), 2.**-60, np.nextafter(2.**-60, np.inf)):
            triangle = np.array([[0., 0., 0.], [1., 0., 0.], [0., height, 0.]])
            for axes in itertools.permutations(range(3)):
                current = triangle[:, axes]
                if height < 2.**-60:
                    for order in itertools.permutations(range(3)):
                        with self.assertRaisesRegex(ValueError, "conditioning"):
                            mesh(current[list(order)], [[0, 1, 2]])
                else:
                    self.compare_permutations(current, np.array([[.25, height / 4, .001]])[:, axes])

    def test_original_wide_plane_counterexample_is_unsupported(self):
        triangle = np.array([[-7.89206285195245e44, -3.849400548750134e44, 1.7649770144202662e44],
                             [9.745720690646674e44, 4.621724450042919e44, 2.266230085741461e43],
                             [-1.8536578386942242e44, -7.723239012927853e43, -1.9916000229944123e44]])
        for axis in range(3):
            self.assertEqual(sum(Fraction(float(x)) for x in triangle[:, axis]), 0)
        for order in itertools.permutations(range(3)):
            with self.assertRaisesRegex(ValueError, "absolute coordinate cap"):
                mesh(triangle[list(order)], [[0, 1, 2]])


if __name__ == "__main__":
    unittest.main()
