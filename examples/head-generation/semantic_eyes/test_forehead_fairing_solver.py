"""Constrained-objective gradients do not redefine geometric acceptance."""
import unittest
import numpy as np
from forehead_fairing_solver import make_forehead_fairing_objective
from forehead_guide_fairing import forehead_polyline_fairing


class PlaneBatch:
    def __init__(self, boundary=False):
        self.boundary = boundary

    def sample_differential(self, coordinates):
        points = np.c_[coordinates, np.zeros(len(coordinates))]
        jacobian = np.tile(np.array([[1., 0.], [0., 1.], [0., 0.]])[None],
                           (len(coordinates), 1, 1))
        return points, jacobian, np.full(len(coordinates), self.boundary)


class ForeheadFairingSolverTests(unittest.TestCase):
    def fixture(self, boundary=False):
        xy = np.array([[0., 0.], [.01, .001], [.02, -.001], [.03, 0.]])
        start = xy.ravel()*1000
        batch = PlaneBatch(boundary)
        def evaluate(values):
            return (batch.sample_differential(values.reshape(-1, 2)/1000)[0],)
        return start, make_forehead_fairing_objective(
            evaluate, batch, list(range(4)), {'guide': [0, 1, 2, 3]},
            {'guide': 30.}, start)

    def test_parameter_gradient_and_donor_boundary_fallback(self):
        for boundary in (False, True):
            start, (value, derivative, metadata) = self.fixture(boundary)
            actual = derivative(start)
            expected = []
            for column in range(len(start)):
                plus, minus = start.copy(), start.copy()
                plus[column] += 1e-4
                minus[column] -= 1e-4
                expected.append((value(plus)-value(minus))/2e-4)
            np.testing.assert_allclose(actual, expected, atol=1e-7, rtol=1e-7)
            self.assertEqual(metadata['donor_fallback_columns'] > 0, boundary)

    def test_bending_is_scale_invariant_and_straight_is_zero(self):
        t = np.linspace(0., np.pi/2, 21)
        curve = np.c_[np.cos(t), np.sin(t), np.zeros(len(t))]
        energy, _, _ = forehead_polyline_fairing(curve, np.pi/2)
        scaled, _, _ = forehead_polyline_fairing(curve*3, 3*np.pi/2)
        self.assertAlmostEqual(energy, scaled, places=12)
        line = np.c_[t, np.zeros((len(t), 2))]
        self.assertEqual(forehead_polyline_fairing(line, np.pi/2)[0], 0.)

    def test_zero_length_edges_are_rejected(self):
        with self.assertRaisesRegex(ValueError, 'Zero-length'):
            forehead_polyline_fairing(np.zeros((3, 3)), 1.)


if __name__ == '__main__':
    unittest.main()
