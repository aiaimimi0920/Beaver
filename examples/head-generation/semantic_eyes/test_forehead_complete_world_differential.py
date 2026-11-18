from forehead_complete_world_differential import *
import unittest
import numpy as np
from forehead_complete_curve_metrics import forehead_complete_side_curve
from forehead_finite_rail_gap import forehead_finite_rail_profile
from forehead_complete_guide_stage import ForeheadCompleteGuideFairingStage
from test_forehead_complete_guide_stage import fixture

class Gradients(unittest.TestCase):

    def direction(self, f, p, g, seed=3):
        v = np.random.default_rng(seed).normal(size=p.shape)
        v /= np.linalg.norm(v)
        eps = 1e-07
        numeric = (f(p + eps * v) - f(p - eps * v)) / (2 * eps)
        analytic = float(np.sum(g * v))
        self.assertAlmostEqual(numeric, analytic, delta=2e-05 * max(1, abs(numeric), abs(analytic)))

    def test_curves(self):
        for seed in range(12):
            rng = np.random.default_rng(seed)
            p = np.cumsum(rng.uniform(0.01, 0.04, (8, 3)), axis=0)
            for frame in [None, np.array([[1, 0, 0], [0, 0, 1]]), np.array([[0, 1, 0], [0, 0, 1]])]:
                value, g = forehead_cd_variation(p, frame)
                self.direction(lambda x: forehead_cd_variation(x, frame)[0], p, g, seed)
                if frame is not None:
                    self.assertAlmostEqual(value, forehead_complete_side_curve(p, *frame)['curvature_variation_energy'], delta=1e-10 * max(value, 1))

    def test_scale(self):
        p = np.random.default_rng(73).normal(size=(7, 3))
        v, g = forehead_cd_variation(p)
        w, h = forehead_cd_variation(p * 7)
        np.testing.assert_allclose(w, v, rtol=2e-13)
        np.testing.assert_allclose(h, g / 7, rtol=2e-12, atol=1e-10)

    def test_translation(self):
        p = np.random.default_rng(18).normal(size=(7, 3))
        v, g = forehead_cd_variation(p)
        np.testing.assert_allclose(g.sum(0), 0, atol=1e-09)

    def test_collapsed(self):
        with self.assertRaises(ValueError):
            forehead_cd_variation(np.zeros((5, 3)))

    def test_branchcut(self):
        with self.assertRaises(ValueError):
            forehead_cd_variation(np.array([[0, 0, 0], [1, 0, 0], [0, 0, 0], [1, 0, 0]]), np.array([[1, 0, 0], [0, 1, 0]]))

    def test_weighted_cv(self):
        x = np.array([1.0, 2.0, 4.0, 3.0])
        w = np.array([2.0, 3.0, 1.0, 5.0])
        v, gx, gw = forehead_cd_weighted_cv(x, w)
        self.direction(lambda z: forehead_cd_weighted_cv(z[:4], z[4:])[0], np.r_[x, w], np.r_[gx, gw])

    def test_support(self):
        p = np.array([[0.0, 0, 0], [1.0, 0.1, 0], [1.2, 1.4, 0.1], [0.1, 1, 0], [2.1, 0.2, 0.2], [2.2, 1.3, 0.1]])
        qs = [[0, 1, 2, 3], [1, 4, 5, 2]]
        v, g, flags = forehead_cd_support_cv(p, qs)
        self.assertFalse(flags)
        self.direction(lambda z: forehead_cd_support_cv(z, qs)[0], p, g)

    def test_uniform_support_fallback(self):
        p = np.array([[0.0, 0, 0], [1.0, 0, 0], [1.0, 1, 0], [0.0, 1, 0]])
        v, g, flags = forehead_cd_support_cv(p, [[0, 1, 2, 3]])
        self.assertEqual(flags, {0, 1, 2, 3})

    def test_gap_random(self):
        successes = 0
        for seed in range(100):
            rng = np.random.default_rng(seed + 71)
            a = rng.normal(size=(4, 3))
            b = rng.normal(size=(5, 3)) + [4, 0, 0]
            v, g, flags, _ = forehead_cd_rail_cv(a, b)
            expected = forehead_finite_rail_profile(a, b)['length_weighted_gap_CV']
            self.assertAlmostEqual(v, expected, delta=1e-12)
            if not flags:
                self.direction(lambda z: forehead_cd_rail_cv(z[:4], z[4:])[0], np.r_[a, b], g, seed)
                successes += 1
        self.assertGreater(successes, 10)

    def test_parallel_fallback(self):
        a = np.array([[0.0, 0, 0], [1.0, 0, 0], [2.0, 0, 0]])
        b = a + [0, 1, 0]
        v, g, flags, _ = forehead_cd_rail_cv(a, b)
        self.assertTrue(flags)

    def test_chain(self):
        g = np.arange(18.0).reshape(6, 3)
        J = np.ones((2, 3, 2))
        v, f = forehead_cd_chain(g, [1, 4], J, [4], [False, False])
        np.testing.assert_equal(f, [False, False, True, True])
        np.testing.assert_equal(v, [g[1].sum()] * 2 + [g[4].sum()] * 2)

    def test_stage_primal(self):
        for count, w, d in [(5, 0.035, 0.012), (8, 0.06, 0.018)]:
            p, prop, conf = fixture(count, w, d)
            stage = ForeheadCompleteGuideFairingStage(prop, p, 'source', conf)
            v, g, flags, detail = forehead_cd_objective(stage, p)
            actual = stage.energy_from(stage.measure(p))
            self.assertEqual(v, actual)
            self.assertLessEqual(detail['analytic_primal_abs_difference'], 1e-11 * max(actual, 1))
            directions = np.random.default_rng(2).normal(size=p.shape)
            directions[flags] = 0
            directions /= max(np.linalg.norm(directions), 1)
            eps = 1e-08
            fd = (stage.energy_from(stage.measure(p + eps * directions)) - stage.energy_from(stage.measure(p - eps * directions))) / (2 * eps)
            self.assertAlmostEqual(fd, float(np.sum(g * directions)), delta=5e-05 * max(abs(fd), 1))
if __name__ == '__main__':
    unittest.main()
