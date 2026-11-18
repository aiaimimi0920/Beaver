"""Synthetic finite-query equivalence, thresholds and deterministic witnesses."""
from pathlib import Path
import sys
import unittest

import numpy as np
from test_support import mesh
from oracle_decimal import decimal_triangle

# Existing pure CPU reference code, NOT any character/model fixture.
sys.path.insert(0, str(Path(__file__).resolve().parents[3]
                      / "examples/head-generation/semantic_eyes"))
from chin_source_shape import chin_shape_exact_distance
from forehead_exact_distance import forehead_exact_distance


class GeometryTests(unittest.TestCase):
    def compare(self, vertices, triangles, queries, legacy=True):
        with mesh(vertices, triangles) as index:
            fast, counts = index.query(queries, True, statistics=True)
            full, full_counts = index.query(queries, True, exhaustive=True, statistics=True)
            repeated = index.query(queries, True)
        for a, b, c in zip(fast, full, repeated):
            np.testing.assert_array_equal(a, b)
            np.testing.assert_array_equal(a, c)
        self.assertTrue(np.all(counts <= len(triangles)))
        self.assertTrue(np.all(full_counts == len(triangles)))
        if legacy:
            for reference in (chin_shape_exact_distance, forehead_exact_distance):
                old = reference(queries, vertices, triangles, True)
                # Test arithmetic comparison budget only; NEVER an admission epsilon.
                scale = max(1.0, abs(vertices).max(), abs(queries).max(initial=0))
                self.assertLessEqual(abs(fast[0] - old[0]).max(initial=0), 2e-13 * scale)
                self.assertLessEqual(abs(fast[2] - old[2]).max(initial=0), 2e-12 * scale)
                np.testing.assert_array_equal(fast[0] > .001, old[0] > .001)
        return fast

    def test_random_legacy_and_exhaustive(self):
        rng = np.random.default_rng(2026110201)
        for _ in range(12):
            vertices = rng.normal(size=(45, 3)) * .01
            triangles = np.arange(45).reshape(-1, 3)
            queries = np.r_[rng.normal(size=(80, 3)) * .01,
                            vertices, vertices[triangles].mean(1)]
            self.compare(vertices, triangles, queries)

    def test_threshold_neighbors_and_duplicate_ties(self):
        vertices = np.array([[0., 0., 0.], [1., 0., 0.], [0., 1., 0.]])
        triangles = np.array([[0, 1, 2], [0, 1, 2]])
        heights = [np.nextafter(.001, 0), .001, np.nextafter(.001, np.inf), 0, 1e-16]
        queries = np.array([[x, y, h] for x, y in
                            [(0., 0.), (.25, .25), (.5, .5), (.5, 0.), (-.1, .5)]
                            for h in heights])
        distance, ids, _ = self.compare(vertices, triangles, queries)
        self.assertTrue(np.all(ids == 0))
        np.testing.assert_array_equal(distance[:3] > .001, [False, False, True])

    def test_distinct_equidistant_triangles(self):
        vertices = np.array([[0., 0., -.001], [1., 0., -.001], [0., 1., -.001],
                             [0., 0., .001], [1., 0., .001], [0., 1., .001]])
        triangles = np.array([[0, 1, 2], [3, 4, 5], [2, 1, 0], [0, 1, 2]])
        # More than a leaf: ties must not depend on traversal/partition order.
        triangles = np.tile(triangles, (9, 1))
        result = self.compare(vertices, triangles, np.array([[.25, .25, 0.]]))
        self.assertEqual(result[0][0], .001)
        self.assertEqual(result[1][0], 0)

    def test_degenerates_slivers_and_tiny_scale(self):
        specials = [np.zeros((3, 3)), np.array([[0., 0., 0.], [1., 0., 0.], [.5, 0., 0.]]),
                    np.array([[0., 0., 0.], [1., 0., 0.], [1., 1e-15, 0.]]),
                    np.array([[0., 0., 0.], [1e-100, 0., 0.], [0., 1e-100, 0.]])]
        for triangle in specials:
            scale = max(abs(triangle).max(), 1e-100)
            points = np.array([[.3, .1, .001], [.7, 0., 0.],
                               [0., 0., 0.], [1., 1e-16, 0.]]) * scale
            actual = self.compare(triangle, np.array([[0, 1, 2]]), points, False)
            oracle = [decimal_triangle(triangle, p)[0] for p in points]
            self.assertLessEqual(abs(actual[0] - oracle).max(), 1e-14 * scale)

    def test_large_translations_and_pruning(self):
        rng = np.random.default_rng(2026110202)
        for offset in (1e8, 1e12, 1e100):
            scale = max(1., offset * 1e-14)
            vertices = rng.normal(size=(90, 3)) * scale + offset
            queries = rng.normal(size=(80, 3)) * scale + offset
            # Deliberate V5 support narrowing, not a claim these historical
            # large-coordinate successes establish general accuracy there.
            with self.assertRaisesRegex(ValueError, "absolute coordinate cap"):
                mesh(vertices, np.arange(90).reshape(-1, 3))
        for offset in (1., 8., 15.):
            vertices = rng.normal(size=(90, 3)) * .01 + offset
            queries = rng.normal(size=(80, 3)) * .01 + offset
            self.compare(vertices, np.arange(90).reshape(-1, 3), queries, False)
        vertices = rng.normal(size=(300, 3)) * .01
        triangles = np.r_[np.arange(300).reshape(-1, 3),
                           [[0, 0, 0], [1, 2, 1], [0, 1, 2], [2, 1, 0]]]
        self.compare(vertices, triangles, rng.normal(size=(500, 3)) * .02, False)

    def test_pruning_actually_occurs(self):
        vertices = np.array([[j * .1, x, y] for j in range(100)
                             for x, y in [(0., 0.), (1., 0.), (0., 1.)]])
        with mesh(vertices, np.arange(300).reshape(-1, 3)) as index:
            (distance, _, _), counts = index.query([[0., .25, .25]], True, statistics=True)
        self.assertEqual(distance[0], 0.)
        self.assertLess(counts[0], 100)


if __name__ == "__main__":
    unittest.main()
