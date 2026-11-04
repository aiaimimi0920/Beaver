"""Same-batch selection contracts on synthetic affine author sheets."""

import unittest
import warnings

import numpy as np

from forehead_matched_gap_jacobian import forehead_gap_source_differential
from forehead_surface_batch import ForeheadSurfaceBatch


def surface(capacity=4096, overlapping=False):
    square = np.array([[0., 0., 0.], [1., 0., 0.],
                       [1., 0., 1.], [0., 0., 1.]])
    chart = np.vstack((square, square if overlapping else square + [1., 0., 0.]))
    world = chart.copy()
    world[:4, 1] = 2 * chart[:4, 0] + 3 * chart[:4, 2]
    world[4:, 1] = 5 * chart[4:, 0] + 7 * chart[4:, 2] + 11
    return ForeheadSurfaceBatch(chart, world, np.arange(8).reshape(2, 4), capacity)


def same_batch_reference(batch, query):
    """Original global argsort and last occurrence, not a canonical tie rule."""
    keys = [(query.dtype.str, row.tobytes()) for row in query]
    batch._fill_candidates(query, keys)
    entries = [batch.candidate_cache[key] for key in keys]
    owners = np.repeat(np.arange(len(query)), [len(item[0]) for item in entries])
    quad = np.concatenate([item[0] for item in entries])
    weights = np.concatenate([item[1] for item in entries])
    margin = np.concatenate([item[2] for item in entries])
    order = np.argsort(margin)
    positions = np.full((len(query), 3), np.nan)
    # Vectorized evaluation keeps the original dtype and floating-point path.
    candidates = np.einsum('ij,ijk->ik', weights[order], batch.vertices[quad[order]])
    positions[owners[order]] = candidates
    selected = np.array([next(int(k) for k in reversed(order) if owners[k] == i)
                         for i in range(len(query))], dtype=int)
    derivative = np.empty((len(query), 3, 2))
    for i, selected_quad in enumerate(quad[selected]):
        slope = (2., 3.) if selected_quad == 0 else (5., 7.)
        derivative[i] = [[1., 0.], list(slope), [0., 1.]]
    return positions, derivative, margin[selected] <= 1e-7


class PairedSurfaceSelectionTests(unittest.TestCase):
    def assert_reference(self, batch, query):
        expected = same_batch_reference(batch, query)
        actual = batch.sample_differential(query)
        self.assertIsNotNone(actual)
        np.testing.assert_array_equal(actual[0], expected[0])
        np.testing.assert_allclose(actual[1], expected[1], rtol=0, atol=2e-14)
        np.testing.assert_array_equal(actual[2], expected[2])
        self.assertEqual(actual[0].tobytes(), batch.sample(query).tobytes())
        self.assertLessEqual(len(batch.candidate_cache), batch.cache_limit)

    def test_original_global_sort_for_reordered_and_repeated_batches(self):
        query = np.array([[1., y] for y in np.linspace(0., 1., 41)]
                         + [[.2, .3], [1.6, .7], [1.-5e-9, .4], [1.+5e-9, .6]])
        for capacity in (0, 2, 4096):
            for current in (query, query[::-1], query[[0, 0, 8, 40, 8, 42]],
                            query.astype(np.float32)):
                with self.subTest(capacity=capacity, length=len(current), dtype=current.dtype):
                    self.assert_reference(surface(capacity), current)

    def test_distinct_boundary_payloads_keep_same_batch_legacy_selection(self):
        query = np.array([[0., y] for y in np.linspace(.01, .99, 50)])
        for current in (query, query[::-1], query[[0, 0, 9, 9, 22]],
                        np.vstack((query, [[1., .5]]))):
            self.assert_reference(surface(overlapping=True), current)

    def test_positions_jacobians_and_flags_follow_one_selected_candidate(self):
        rng = np.random.default_rng(713)
        for _ in range(40):
            query = np.column_stack((np.ones(25), rng.uniform(0., 1., 25)))
            query = np.vstack((query, [[.25, .25], [1.75, .75]]))
            query = query[rng.permutation(len(query))]
            self.assert_reference(surface(), query)

    def test_missing_and_overlapping_interiors_remain_invalid(self):
        for query in (np.array([[.4, .6]]), np.array([[4., 4.]]),
                      np.array([[np.nan, 0.]]), np.array([[0., np.inf]])):
            batch = surface(overlapping=True)
            self.assertIsNone(batch.sample(query))
            self.assertIsNone(batch.sample_differential(query))

    def test_one_missing_query_invalidates_whole_batch(self):
        query = np.array([[.3, .4], [1., .5], [4., 4.]])
        self.assertIsNone(surface().sample(query))
        self.assertIsNone(surface().sample_differential(query))

    def test_empty_batch_keeps_integer_selection_and_public_shapes(self):
        batch = surface(0)
        positions, derivative, flags = batch.sample_differential(np.empty((0, 2)))
        self.assertEqual(positions.shape, (0, 3))
        self.assertEqual(derivative.shape, (0, 3, 2))
        self.assertEqual(flags.shape, (0,))
        self.assertEqual(flags.dtype, np.dtype(bool))
        self.assertEqual(batch.sample(np.empty((0, 2))).shape, (0, 3))

    def test_live_world_changes_update_positions_and_derivatives(self):
        batch = surface(2)
        query = np.array([[1., .5], [.25, .25], [1.75, .75]])
        before = batch.sample_differential(query)
        batch.vertices *= 2
        after = batch.sample_differential(query)
        np.testing.assert_array_equal(after[0], 2 * before[0])
        np.testing.assert_array_equal(after[1], 2 * before[1])
        np.testing.assert_array_equal(after[2], before[2])

    def test_original_float64_position_cast_is_retained(self):
        query = np.array([[1., .5], [.25, .25]])
        for dtype in (np.float32, np.float64, np.longdouble, np.complex128):
            batch = surface()
            batch.vertices = batch.vertices.astype(dtype)
            if dtype == np.complex128:
                batch.vertices += .25j
            with warnings.catch_warnings():
                warnings.simplefilter('ignore', np.exceptions.ComplexWarning)
                positions = batch.sample(query)
                differential = batch.sample_differential(query)
            self.assertEqual(positions.dtype, np.dtype(np.float64))
            self.assertEqual(positions.tobytes(), differential[0].tobytes())

    def test_smooth_jacobians_match_independent_two_sided_probes(self):
        batch = surface()
        query = np.array([[.2, .4], [1.7, .6]])
        _, derivative, flags = batch.sample_differential(query)
        self.assertFalse(flags.any())
        for axis in range(2):
            delta = np.zeros_like(query)
            delta[:, axis] = 1e-6
            numeric = (batch.sample(query + delta) - batch.sample(query - delta)) / 2e-6
            np.testing.assert_allclose(derivative[:, :, axis], numeric, rtol=1e-8, atol=1e-9)

    def test_gap_adapter_preserves_primal_flags_and_converts_units_once(self):
        query = np.array([[1., .5], [.25, .25]])
        batch = surface(0)
        primal, derivative, flags = batch.sample_differential(query)
        actual = forehead_gap_source_differential(batch, query)
        np.testing.assert_array_equal(actual[0], primal)
        np.testing.assert_array_equal(actual[1], derivative / 1000)
        np.testing.assert_array_equal(actual[2], flags)
        with self.assertRaisesRegex(ValueError, 'Invalid author chart'):
            forehead_gap_source_differential(batch, np.array([[9., 9.]]))


if __name__ == '__main__':
    unittest.main()
