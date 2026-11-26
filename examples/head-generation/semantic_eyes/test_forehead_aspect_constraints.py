"""Aspect feasibility is unchanged; ties no longer select one compressed branch."""
import unittest
import numpy as np
from forehead_aspect_constraints import forehead_aspect_reference, forehead_pairwise_aspect
from forehead_fit_density import forehead_constraint_layout


class ForeheadAspectConstraintTests(unittest.TestCase):
    def test_equivalent_feasible_set(self):
        rng = np.random.default_rng(44)
        for _ in range(200):
            points = rng.normal(size=(4, 3))
            lengths = np.linalg.norm(np.roll(points, -1, axis=0)-points, axis=1)
            for bound in (1., 2., 6.998, 7.):
                rows = forehead_pairwise_aspect(points, [[0, 1, 2, 3]], bound)
                self.assertEqual(bool(rows.min() >= 0), bool(lengths.max()/lengths.min() <= bound))

    def test_exact_bound_rectangles(self):
        for length in (6.999, 7., 7.001):
            points = np.array([[0., 0., 0.], [length, 0., 0.], [length, 1., 0.], [0., 1., 0.]])
            self.assertEqual(bool(forehead_pairwise_aspect(points, [[0, 1, 2, 3]]).min() >= 0), length <= 7)

    def test_tied_minimum_direction_and_world_gradient(self):
        points = np.array([[0., 0., 0.], [7., 0., 0.], [7., 1., 0.], [0., 1., 0.]])
        quads = [[0, 1, 2, 3]]
        scale = forehead_aspect_reference(points, quads)
        _, gradient, pairs = forehead_pairwise_aspect(points, quads, reference_squared_length=scale, gradient=True)
        self.assertEqual(len(set(map(tuple, pairs))), 12)
        for column in range(12):
            plus, minus = points.copy(), points.copy()
            plus.flat[column] += 1e-6
            minus.flat[column] -= 1e-6
            fd = (forehead_pairwise_aspect(plus, quads, reference_squared_length=scale)-
                  forehead_pairwise_aspect(minus, quads, reference_squared_length=scale))/2e-6
            np.testing.assert_allclose(fd, gradient[0, :, column//3, column % 3], atol=2e-8)
        direction = np.array([[0., 1., 0.], [0., -1., 0.], [0., 0., 0.], [0., 0., 0.]])
        predicted = np.einsum('rvc,vc->r', gradient[0], direction)
        for sign in (-1, 1):
            h = 1e-5
            actual = (forehead_pairwise_aspect(points+sign*h*direction, quads, reference_squared_length=scale)-
                      forehead_pairwise_aspect(points, quads, reference_squared_length=scale))/(sign*h)
            np.testing.assert_allclose(actual, predicted, atol=2e-4)

    def test_fixed_positive_scale_and_quad_major_order(self):
        points = np.array([[0., 0., 0.], [2., 0., 0.], [2., 1., 0.], [0., 1., 0.]])
        quads = [[0, 1, 2, 3], [3, 2, 1, 0]]
        raw = forehead_pairwise_aspect(points, quads).reshape(2, 12)
        scaled = forehead_pairwise_aspect(points, quads, reference_squared_length=[2., 3.]).reshape(2, 12)
        np.testing.assert_allclose(raw, scaled*np.array([2., 3.])[:, None])
        self.assertTrue(np.array_equal(raw >= 0, scaled >= 0))

    def test_legacy_layout_default_and_twelve_row_ownership(self):
        legacy = forehead_constraint_layout(2, 3, 4)
        self.assertEqual(legacy['aspect'], (16, 18))
        self.assertEqual(legacy['orientation'], (18, 26))
        new = forehead_constraint_layout(2, 3, 4, aspect_rows_per_quad=12)
        self.assertEqual(new['aspect'], (16, 40))
        self.assertEqual(new['orientation'], (40, 48))
        self.assertEqual(new['field'], (48, 51))
        self.assertEqual(new['count'], legacy['count']+22)
        for invalid in (0, -1, 1.5, True):
            with self.assertRaises(ValueError):
                forehead_constraint_layout(2, 3, 4, aspect_rows_per_quad=invalid)

    def test_degenerate_nonfinite_and_invalid_scaling_fail_closed(self):
        points = np.array([[0., 0., 0.], [1., 0., 0.], [1., 1., 0.], [0., 1., 0.]])
        for scale in ([0.], [-1.], [np.nan], [1., 2.]):
            with self.assertRaises(ValueError):
                forehead_pairwise_aspect(points, [[0, 1, 2, 3]], reference_squared_length=scale)
        points[1] = points[0]
        with self.assertRaises(ValueError):
            forehead_pairwise_aspect(points, [[0, 1, 2, 3]])
        points[1, 0] = np.nan
        with self.assertRaises(ValueError):
            forehead_aspect_reference(points, [[0, 1, 2, 3]])


if __name__ == '__main__':
    unittest.main()
