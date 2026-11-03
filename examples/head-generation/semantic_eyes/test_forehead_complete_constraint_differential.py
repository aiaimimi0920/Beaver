from forehead_complete_constraint_differential import *
import unittest
import numpy as np
from test_forehead_complete_guide_stage import fixture
from forehead_complete_guide_stage import ForeheadCompleteGuideFairingStage

class Constraints(unittest.TestCase):

    def test_partial_mask_coloring_counterexample(self):
        from scipy.optimize._numdiff import approx_derivative, group_columns
        from scipy.sparse import csr_matrix
        x = np.array([1.0, 2.0])
        f = lambda z: np.array([2 * z[0] + 3 * z[1]])
        unsafe = csr_matrix([[1, 0]])
        wrong = approx_derivative(f, x, method='2-point', abs_step=0.0001, sparsity=(unsafe, group_columns(unsafe))).toarray()
        self.assertGreater(abs(wrong[0, 0] - 2), 1)
        safe = approx_derivative(lambda z: f(np.array([z[0], x[1]])), x[:1], method='2-point', abs_step=0.0001)
        self.assertAlmostEqual(float(safe.ravel()[0]), 2, places=8)

    def test_near_extremum_stays_fallback_both_sides(self):
        gradients = np.array([[[1.0, 0, 0]], [[0, 1.0, 0]]])
        for delta in (-1e-12, 0, 1e-12):
            for maximum in (True, False):
                self.assertTrue(forehead_cd_extremum([1, 1 + delta], gradients, maximum=maximum, branch_radius=1e-07)[2])

    def test_unique_extremum(self):
        self.assertFalse(forehead_cd_extremum([1.0, 2.0], np.array([[[1.0, 0, 0]], [[0, 1.0, 0]]]), branch_radius=1e-07)[2])

    def test_peak_zero_and_collapse(self):
        line = np.c_[np.arange(4.0), np.zeros((4, 2))]
        self.assertTrue(forehead_cd_peak(line)[2])
        self.assertTrue(forehead_cd_peak(line, np.array([[1.0, 0, 0], [0, 1, 0]]))[2])
        with self.assertRaises(ValueError):
            forehead_cd_peak(np.zeros((4, 3)))

    def test_gap_zero_rejected(self):
        line = np.c_[np.arange(4.0), np.zeros((4, 2))]
        with self.assertRaises(ValueError):
            forehead_cd_rail_cv(line, line)

    def test_metric_rows_directions(self):
        p, prop, conf = fixture(6, 0.04, 0.014)
        stage = ForeheadCompleteGuideFairingStage(prop, p, 'test', conf)
        values, g, flags, deps, error = forehead_cd_metric_rows(stage, p, branch_radius=1e-07)
        self.assertTrue(np.array_equal(values, stage.measure(p)))
        rng = np.random.default_rng(19)
        for r in range(len(values)):
            direction = rng.normal(size=p.shape)
            direction[list(flags[r])] = 0
            direction /= max(np.linalg.norm(direction), 1)
            h = 1e-08
            numerical = (stage.measure(p + h * direction)[r] - stage.measure(p - h * direction)[r]) / (2 * h)
            analytic = np.sum(g[r] * direction)
            self.assertAlmostEqual(numerical, analytic, delta=0.0002 * max(1, abs(numerical), abs(analytic)))
            outside = sorted(set(range(len(p))) - deps[r])
            self.assertEqual(np.count_nonzero(g[r, outside]), 0)
if __name__ == '__main__':
    unittest.main()
