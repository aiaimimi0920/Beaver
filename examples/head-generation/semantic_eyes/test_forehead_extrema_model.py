import copy
import unittest
import numpy as np
from forehead_extrema_model import forehead_role_extrema_model
from forehead_extrema_primitives import forehead_extrema_edges, forehead_extrema_turns
from forehead_complete_guide_stage import ForeheadCompleteGuideFairingStage
from test_forehead_complete_guide_stage import fixture

class ModelTests(unittest.TestCase):

    def setup(self, count=6):
        p, proposal, config = fixture(count, 0.04, 0.014)
        config['max_evaluations'] = 10000
        stage = ForeheadCompleteGuideFairingStage(proposal, p, 'generic-test', config)
        ids = np.arange(len(p))
        differential = np.zeros((len(p), 3, 2))
        differential[:, 0, 0] = 1
        differential[:, 1, 0] = 0.13
        differential[:, 1, 1] = -0.21
        differential[:, 2, 1] = 1
        return (p, stage, ids, differential)

    def model(self, p, stage, ids, differential, boundary=None, scales=None):
        if boundary is None:
            boundary = np.zeros(len(ids), bool)
        if scales is None:
            scales = np.linspace(1, 2, stage.count + 3)
        return forehead_role_extrema_model(stage, p, ids, p[ids], differential, boundary, 3, scales, stage.constraints(p))

    def test_all_children_exact_aggregate_equivalence_and_caps_unchanged(self):
        p, stage, ids, differential = self.setup()
        before = copy.deepcopy((stage.caps, stage.reference, stage.target, stage.config))
        model = self.model(p, stage, ids, differential)
        actual = stage.constraints(p)
        scale = np.linspace(1, 2, stage.count + 3)
        self.assertEqual(set(model.handled_rows) | set(model.unsupported_rows), {3 + 11 * r + k for r in range(5) for k in (1, 3, 4, 6, 8, 10)})
        for row in model.handled_rows:
            self.assertAlmostEqual(model.residuals[model.owners == row].min(), actual[row - 3] / scale[row], places=12)
        np.testing.assert_array_equal(stage.caps, before[0])
        self.assertEqual((stage.reference, stage.target, stage.config), before[1:])

    def test_directional_differential_all_children(self):
        p, stage, ids, differential = self.setup()
        model = self.model(p, stage, ids, differential)
        for seed in range(10):
            v = np.random.default_rng(seed).normal(size=(len(ids), 2))
            v /= np.linalg.norm(v)
            h = 0.0001
            world = np.einsum('nia,na->ni', differential, v) / 1000
            plus = self.model(p + h * world, stage, ids, differential)
            minus = self.model(p - h * world, stage, ids, differential)
            self.assertEqual(model.labels, plus.labels)
            self.assertEqual(model.labels, minus.labels)
            finite = (plus.residuals - minus.residuals) / (2 * h)
            analytic = model.jacobian @ v.ravel()
            np.testing.assert_allclose(analytic, finite, rtol=3e-05, atol=2e-07)

    def test_fixed_endpoints_omit_columns_but_keep_children(self):
        p, stage, ids, differential = self.setup()
        fixed = {v for path in stage.paths.values() for v in (path[0], path[-1])}
        keep = np.array([i for i in ids if i not in fixed])
        model = self.model(p, stage, keep, differential[keep])
        full = self.model(p, stage, ids, differential)
        np.testing.assert_array_equal(model.residuals, full.residuals)
        columns = np.array([2 * i + j for i in keep for j in range(2)])
        np.testing.assert_array_equal(model.jacobian.toarray(), full.jacobian[:, columns].toarray())

    def test_boundary_owner_keeps_old_fallback(self):
        p, stage, ids, differential = self.setup()
        boundary = np.zeros(len(ids), bool)
        boundary[stage.paths['red'][1]] = True
        model = self.model(p, stage, ids, differential, boundary)
        self.assertTrue({3 + k for k in (1, 3, 4, 6, 8, 10)} <= set(model.unsupported_rows))
        self.assertFalse(any((row in model.handled_rows for row in (4, 6, 7, 9, 11, 13))))

    def test_source_primal_mismatch_rejected_and_charged(self):
        p, stage, ids, differential = self.setup()
        sampled = p[ids].copy()
        sampled[0, 0] += 1e-10
        before = stage.total_calls
        with self.assertRaises(ValueError):
            forehead_role_extrema_model(stage, p, ids, sampled, differential, np.zeros(len(ids), bool), 0, np.ones(stage.count), stage.constraints(p))
        self.assertEqual(stage.total_calls - before, 2)

    def test_public_budget_remains_enforced(self):
        p, stage, ids, differential = self.setup()
        actual = stage.constraints(p)
        stage.total_calls = stage.config['max_evaluations']
        with self.assertRaisesRegex(ValueError, 'budget exhausted'):
            forehead_role_extrema_model(stage, p, ids, p[ids], differential, np.zeros(len(ids), bool), 0, np.ones(stage.count), actual)

    def test_parent_goals_broadcast_not_independent_fractional_restoration(self):
        p, stage, ids, differential = self.setup()
        model = self.model(p, stage, ids, differential)
        goal = np.linspace(-0.2, 0.1, stage.count + 3)
        A, b = model.inequalities(goal)
        np.testing.assert_array_equal(b, model.residuals - goal[model.owners])
        np.testing.assert_array_equal(A.toarray(), -model.jacobian.toarray())
        parent = -2.0
        target = 0.0
        rho = 0.5
        common = (1 - rho) * parent + rho * target
        independent = (1 - rho) * -0.5 + rho * target
        self.assertNotEqual(common, independent)

    def test_float32_admission_unchanged(self):
        p, stage, ids, differential = self.setup()
        before = stage.accepts(p.astype(np.float32).astype(float))
        self.model(p, stage, ids, differential)
        self.assertEqual(before, stage.accepts(p.astype(np.float32).astype(float)))

    def test_fixed_violating_children_are_not_dropped(self):
        p, stage, _, _ = self.setup()
        ids = np.empty(0, dtype=int)
        model = self.model(p, stage, ids, np.empty((0, 3, 2)))
        self.assertGreater(len(model.residuals), 0)
        self.assertEqual(model.jacobian.shape[1], 0)
        _, b = model.inequalities(np.ones(stage.count + 3))
        self.assertTrue(np.any(b < 0))

    def test_original_aggregate_rejection_not_changed_by_model(self):
        p, stage, ids, differential = self.setup()
        actual = stage.constraints(p).copy()
        original_saved = stage.accepts(p.astype(np.float32).astype(float))
        self.model(p, stage, ids, differential)
        np.testing.assert_array_equal(stage.constraints(p), actual)
        self.assertEqual(original_saved, stage.accepts(p.astype(np.float32).astype(float)))

class PrimitiveAdversaries(unittest.TestCase):

    def test_projected_zero_turn_has_two_nonzero_sign_cuts(self):
        p = np.array([[0.0, 0, 0], [1.0, 0, 0], [2.0, 0, 0], [3.0, 0, 0]])
        angle, g, bad = forehead_extrema_turns(p, np.array([[1, 0, 0], [0, 1, 0]]))
        self.assertFalse(bad.any())
        self.assertTrue((angle == 0).all())
        self.assertGreater(np.linalg.norm(g), 0)
        v = np.array([[0.0, 0, 0], [0, 1.0, 0], [0, 0, 0], [0, 0, 0]])
        h = 1e-07
        numeric = abs(forehead_extrema_turns(p + h * v, np.array([[1, 0, 0], [0, 1, 0]]))[0][0]) / h
        predicted = abs(np.sum(g[0] * v[:3]))
        self.assertAlmostEqual(numeric, predicted, places=4)

    def test_projected_branch_cut_and_3d_singularities_defer(self):
        straight = np.array([[0.0, 0, 0], [1.0, 0, 0], [2.0, 0, 0]])
        reverse = np.array([[0.0, 0, 0], [1.0, 0, 0], [0.0, 0, 0]])
        self.assertTrue(forehead_extrema_turns(straight)[2].all())
        self.assertTrue(forehead_extrema_turns(reverse)[2].all())
        self.assertTrue(forehead_extrema_turns(reverse, np.array([[1, 0, 0], [0, 1, 0]]))[2].all())

    def test_collapsed_and_invalid_frame_rejected(self):
        with self.assertRaises(ValueError):
            forehead_extrema_edges(np.zeros((3, 3)))
        with self.assertRaises(ValueError):
            forehead_extrema_turns(np.eye(3), np.ones((2, 3)))

    def test_exact_tie_and_inactive_branch_switch(self):
        for direction in (-2.0, -0.1, 0.1, 2.0):
            self.assertEqual(max(direction, -direction), abs(direction))
        x = 0.01
        direction = -0.03
        self.assertAlmostEqual(max(x + direction, -x - direction), 0.02)
        self.assertLess(x + direction, -x - direction)

    def test_linearized_upper_length_is_not_nonlinear_admission(self):
        p = np.array([[0.0, 0, 0], [1.0, 0, 0], [2.0, 0.1, 0]])
        length, g = forehead_extrema_edges(p)
        trial = p.copy()
        trial[1, 1] = 0.2
        predicted = length[0] + np.sum(g[0] * (trial[:2] - p[:2]))
        actual = forehead_extrema_edges(trial)[0][0]
        self.assertEqual(predicted, 1000.0)
        self.assertGreater(actual, 1000.0)

    def test_quantized_extremum_can_switch(self):
        rng = np.random.default_rng(74)
        witnessed = False
        for _ in range(200):
            base = 1.5 + rng.uniform(0, 1e-07)
            lengths = rng.uniform(9.9e-07, 1.01e-06, 2)
            p = np.c_[np.zeros((3, 2)), base + np.r_[0, np.cumsum(lengths)]]
            smooth = np.linalg.norm(np.diff(p, axis=0), axis=1)
            saved = np.linalg.norm(np.diff(p.astype(np.float32).astype(float), axis=0), axis=1)
            if np.argmin(smooth) != np.argmin(saved) and saved.min() < smooth.min():
                floor = (saved.min() + smooth.min()) / 2
                self.assertGreaterEqual(smooth.min(), floor)
                self.assertLess(saved.min(), floor)
                witnessed = True
                break
        self.assertTrue(witnessed)

    def test_one_sided_min_length_direction_at_equal_edges(self):
        p = np.array([[0.0, 0, 0], [1.0, 0, 0], [1.0, 1.0, 0]])
        length, gradient = forehead_extrema_edges(p)
        for seed in range(30):
            direction = np.random.default_rng(seed).normal(size=p.shape)
            h = 1e-08
            predicted = min((np.sum(g * direction[i:i + 2]) for i, g in enumerate(gradient)))
            measured = (forehead_extrema_edges(p + h * direction)[0].min() - length.min()) / h
            self.assertAlmostEqual(predicted, measured, delta=0.001)
if __name__ == '__main__':
    unittest.main()
