"""Bitwise parity against the original forward candidate-slot implementation."""
import unittest
import warnings
from types import SimpleNamespace
from unittest.mock import patch

import numpy as np

import chin_source_shape as shape


def reference_distance(surface, pts, details=False):
    """Pre-dedup implementation, deliberately independent of the new grouping."""
    out, bestids, hits = [], [], []
    for a in range(0, len(pts), 200):
        p = pts[a:a + 200]
        ns = surface.tree.query(p, k=12)[1]
        ids = surface.inc[ns].reshape(len(p), -1)
        valid = ids >= 0
        tr = surface.tri[np.maximum(ids, 0)]
        pp = np.repeat(p, ids.shape[1], axis=0)
        hit = shape.chin_shape_closest_triangles(
            tr.reshape(-1, 3, 3), pp
        ).reshape(len(p), -1, 3)
        d = np.linalg.norm(hit - p[:, None], axis=2)
        d[~valid] = np.inf
        j = d.argmin(1)
        out.extend(d[np.arange(len(p)), j])
        bestids.extend(ids[np.arange(len(p)), j])
        hits.extend(hit[np.arange(len(p)), j])
    return (np.array(out), np.array(bestids), np.array(hits)) if details else np.array(out)


class FixedTree:
    def query(self, points, k):
        return np.zeros((len(points), k)), np.zeros((len(points), k), int)


class ForwardDistanceParityTests(unittest.TestCase):
    def setUp(self):
        self.vertices = np.array([
            [0., 0., 0.], [1., 0., .05], [1., 1., -.02], [0., 1., .03]
        ])
        self.surface = shape.ChinFrozenDistance(self.vertices, [[0, 1, 2, 3]], 32)

    def assert_parity(self, surface, points):
        snapshots = [value.tobytes() for value in (surface.tri, surface.inc, points)]
        with warnings.catch_warnings():
            warnings.simplefilter('ignore', RuntimeWarning)
            expected = reference_distance(surface, points, True)
            actual = shape.ChinFrozenDistance.distance(surface, points, True)
            self.assertIsInstance(actual, tuple)
            self.assertEqual(len(actual), 3)
            for old, new in zip(expected, actual):
                self.assertEqual(old.dtype, new.dtype)
                self.assertEqual(old.shape, new.shape)
                self.assertEqual(old.tobytes(), new.tobytes())
            plain = shape.ChinFrozenDistance.distance(surface, points)
            self.assertIsInstance(plain, np.ndarray)
            self.assertEqual(expected[0].tobytes(), plain.tobytes())
        self.assertEqual(snapshots, [v.tobytes() for v in (surface.tri, surface.inc, points)])

    def test_seeded_queries_dtype_and_chunk_boundaries(self):
        random = np.random.default_rng(536545)
        for dtype in (np.float32, np.float64):
            points = random.uniform([-.1, -.1, -.1], [1.1, 1.1, .1], (801, 3)).astype(dtype)
            self.assert_parity(self.surface, points[::2])
            for count in (0, 1, 199, 200, 201):
                self.assert_parity(self.surface, points[:count])

    def test_vertices_edges_and_duplicate_quads(self):
        vertices = self.vertices.copy()
        vertices[:, 2] = 0
        surface = shape.ChinFrozenDistance(vertices, [[0, 1, 2, 3]] * 2, 8)
        points = np.array([
            [0., 0., 0.], [1., 1., 0.], [.5, .5, 0.], [.5, 0., 0.],
            [.5, .5, 1e-15], [.5, .5, -1e-15], [.5, .5, .1]
        ])
        self.assert_parity(surface, np.tile(points, (59, 1)))
        self.assert_parity(surface, surface.points[::3])

    def test_original_slot_tie_wins_not_smallest_triangle_id(self):
        triangle = [[0., 0., 0.], [1., 0., 0.], [0., 1., 0.]]
        surface = SimpleNamespace(
            tree=FixedTree(), tri=np.array([triangle, triangle]),
            inc=np.array([[1, 0, 1, -1, 0]])
        )
        points = np.array([[.25, .25, .1]])
        self.assert_parity(surface, points)
        self.assertEqual(shape.ChinFrozenDistance.distance(surface, points, True)[1].tolist(), [1])

    def test_invalid_incidence_and_degenerate_existing_behavior(self):
        triangle = np.array([[[0., 0., 0.], [1., 0., 0.], [0., 1., 0.]]])
        for geometry, incidence in (
            (triangle, [[-1, -1]]),
            (np.zeros((1, 3, 3)), [[0, -1, 0]]),
            (triangle, [[0, -1, 0]])
        ):
            surface = SimpleNamespace(tree=FixedTree(), tri=geometry, inc=np.array(incidence))
            self.assert_parity(surface, np.array([[.2, .3, .1], [0., 0., 0.]]))

    def test_too_small_vertex_tree_preserves_index_error(self):
        surface = shape.ChinFrozenDistance(self.vertices, [[0, 1, 2, 3]], 1)
        points = np.array([[.25, .25, .1]])
        with self.assertRaises(IndexError):
            reference_distance(surface, points)
        with self.assertRaises(IndexError):
            surface.distance(points)

    def test_closest_point_work_is_reduced_and_chunk_bounded(self):
        triangle = np.array([[[0., 0., 0.], [1., 0., 0.], [0., 1., 0.]]])
        surface = SimpleNamespace(tree=FixedTree(), tri=triangle, inc=np.array([[0, -1, 0]]))
        points = np.tile([[.2, .3, .1]], (401, 1))
        calls = []
        original = shape.chin_shape_closest_triangles

        def measured(triangles, queries):
            calls.append(len(queries))
            return original(triangles, queries)

        with patch.object(shape, 'chin_shape_closest_triangles', side_effect=measured):
            shape.ChinFrozenDistance.distance(surface, points)
        # Original evaluates 12 * 3 slots per point; duplicate work alone vanishes.
        self.assertEqual(calls, [200, 200, 1])
        self.assertEqual(sum(calls), len(points))


if __name__ == '__main__':
    unittest.main()
