"""Public differentials retain the legacy sampler, including cache eviction/ties."""
import unittest
import numpy as np
from forehead_surface_batch import ForeheadSurfaceBatch


class ForeheadSurfaceDifferentialTests(unittest.TestCase):
    def fixture(self, limit):
        chart = np.array([[0., 0., 0.], [.02, 0., 0.],
                          [.023, 0., .03], [0., 0., .03]])
        world = np.array([[0., -.1, 1.6], [.02, -.098, 1.601],
                          [.021, -.094, 1.632], [0., -.099, 1.63]])
        return ForeheadSurfaceBatch(chart, world, [[0, 1, 2, 3]], cache_limit=limit)

    def test_differential_primal_is_exact_across_eviction_and_reordering(self):
        points = np.array([[.01, .012], [0., 0.], [.02, 0.], [.01, .012],
                           [.008, .024], [0., .03], [.023, .03]])
        for capacity in (0, 8, 4096):
            batch = self.fixture(capacity)
            for order in (np.arange(len(points)), np.array([6, 4, 2, 1, 0, 3, 5])):
                current = points[order]
                expected = batch.sample(current)
                actual, jacobian, flags = batch.sample_differential(current)
                np.testing.assert_array_equal(actual, expected)
                self.assertEqual(jacobian.shape, (len(points), 3, 2))
                self.assertEqual(flags.dtype, np.dtype(bool))
                self.assertEqual(int(flags.sum()), 4)

    def test_interior_jacobian_matches_independent_central_difference(self):
        batch = self.fixture(8)
        points = np.array([[.01, .012], [.008, .024]])
        _, jacobian, flags = batch.sample_differential(points)
        self.assertFalse(flags.any())
        epsilon = 1e-7
        for axis in range(2):
            positive, negative = points.copy(), points.copy()
            positive[:, axis] += epsilon
            negative[:, axis] -= epsilon
            expected = (batch.sample(positive)-batch.sample(negative))/(2*epsilon)
            np.testing.assert_allclose(jacobian[:, :, axis], expected, rtol=1e-7, atol=2e-9)

    def test_empty_invalid_and_mutated_world_do_not_use_stale_positions(self):
        batch = self.fixture(8)
        values, derivative, flags = batch.sample_differential(np.empty((0, 2)))
        self.assertEqual(values.shape, (0, 3))
        self.assertEqual(derivative.shape, (0, 3, 2))
        self.assertEqual(flags.shape, (0,))
        self.assertIsNone(batch.sample_differential(np.array([[1., 1.]])))
        self.assertIsNone(batch.sample_differential(np.array([[np.nan, 0.]])))
        point = np.array([[.01, .012]])
        before, jacobian, _ = batch.sample_differential(point)
        batch.vertices *= 2
        after, updated, _ = batch.sample_differential(point)
        np.testing.assert_array_equal(after, 2*before)
        np.testing.assert_array_equal(updated, 2*jacobian)


if __name__ == '__main__':
    unittest.main()
