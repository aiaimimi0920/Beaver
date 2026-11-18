"""All-child LP integration preserves original goals, nonlinear checks and budgets."""
import copy
import unittest
from unittest.mock import patch
import numpy as np
from scipy import sparse
from scipy.optimize import OptimizeResult
import forehead_lp_restore as solver
from forehead_extrema_lp import (forehead_extrema_rows, forehead_extrema_rhs,
    forehead_extrema_prediction, forehead_make_extrema_model)
from forehead_extrema_model import ForeheadExtremaLinearModel, forehead_role_extrema_model
from forehead_complete_guide_stage import ForeheadCompleteGuideFairingStage
from forehead_complete_constraint_differential import forehead_cd_parameter_constraints
from forehead_complete_split_jacobian import forehead_complete_fallback_block
from test_forehead_complete_guide_stage import fixture


def model(values, rows, owners=(0, 0), count=2, unsupported=()):
    return ForeheadExtremaLinearModel(np.array(owners, int), np.array(values, float),
        sparse.csr_matrix(rows), tuple(sorted(set(owners))), tuple(unsupported), (), count)


class ExtremaLPTests(unittest.TestCase):
    def solve(self, **options):
        settings = dict(start=np.array([0.]), radii=np.array([1.]),
            constraints=lambda x: np.array([x[0]-.5, 1.]),
            jacobian=lambda x: sparse.csr_matrix([[1.], [0.]]),
            admit=lambda x: bool(x[0] >= .5), orientation_rows=(1, 2),
            domain=lambda x: True, wall_seconds=5, max_iterations=2)
        settings.update(options)
        return solver.forehead_restore_linearized(**settings)

    def test_none_model_has_bitwise_original_rows_rhs_and_prediction(self):
        c = np.array([-.172, 1.])
        J = sparse.csr_matrix([[1., 2.], [0., 0.]])
        values, rows, owners = forehead_extrema_rows(c, J, None)
        rhs = np.array([-.172*.31, 1.])
        np.testing.assert_array_equal(forehead_extrema_rhs(c, values, owners, rhs), rhs)
        self.assertEqual((rows != J).nnz, 0)
        np.testing.assert_array_equal(forehead_extrema_prediction(2, values, rows, owners,
            np.array([.13, .27])), c+J@np.array([.13, .27]))

    def test_common_fractional_goal_in_major_actual_matrix(self):
        calls = []
        def capture(*args, **kwargs):
            calls.append(kwargs)
            return OptimizeResult(success=False, message='captured')
        c = np.array([-2., 1.])
        m = model([-2., -.5, 3.], [[1.], [2.], [3.]], (0, 0, 0))
        with patch.object(solver, 'linprog', side_effect=capture):
            result, metadata = self.solve(constraints=lambda x: c.copy(), admit=lambda x: False,
                reduction_fraction=.25, linear_model=lambda x: m)
        self.assertFalse(result.success)
        # Original parent goal is -1.5, broadcast even to the inactive child.
        np.testing.assert_allclose(calls[0]['b_ub'][:4], [1e8, -.5, .5, 1.5])
        np.testing.assert_allclose(calls[0]['A_ub'].toarray()[1:4, 0], [-1, -1, -1])
        self.assertEqual(metadata['steps'][0]['minimum_original'], -2.)
        self.assertEqual(metadata['steps'][0]['merit_before'], 2.)
        self.assertEqual(calls[0]['options']['primal_feasibility_tolerance'], 1e-9)
        self.assertLessEqual(calls[0]['options']['time_limit'], 5)
        self.assertEqual(calls[0]['bounds'], [(-1., 1.), (0, 1.)])

    def test_supported_rows_replaced_unsupported_and_fixed_children_retained(self):
        c = np.array([-.2, -.3, 1.])
        J = sparse.csr_matrix([[9.], [7.], [0.]])
        m = model([-.2, .4], [[0.], [3.]], count=3, unsupported=(1,))
        values, rows, owners = forehead_extrema_rows(c, J, m)
        np.testing.assert_array_equal(owners, [1, 2, 0, 0])
        np.testing.assert_array_equal(rows.toarray().ravel(), [7, 0, 0, 3])
        np.testing.assert_array_equal(values, [-.3, 1., -.2, .4])
        result, _ = self.solve(constraints=lambda x: np.array([-.2, 1.]),
            admit=lambda x: False, linear_model=lambda x: m.__class__(m.owners,
                m.residuals, m.jacobian, m.handled_rows, (), (), 2))
        self.assertFalse(result.success)
        np.testing.assert_array_equal(result.x, [0])

    def test_invalid_and_missing_ownership_fail_closed(self):
        for m in [model([-.5], [[1.]], (2,)),
                  model([-.4], [[1.]], (0,)),
                  model([-.5], [[1., 0.]], (0,)),
                  model([-.5], [[np.nan]], (0,)),
                  model([-.5], [[1.]], (0,), unsupported=(0,))]:
            with self.subTest(model=m), self.assertRaises(ValueError):
                self.solve(linear_model=lambda x: m)
        with self.assertRaises(ValueError):
            self.solve(linear_model=True)

    def test_alpha_prediction_reduces_after_step_not_before(self):
        c = np.array([0., 1.])
        values, rows, owners = forehead_extrema_rows(c, sparse.csr_matrix([[1.], [0.]]),
            model([0., 1.], [[1.], [-1.]]))
        midpoint = forehead_extrema_prediction(2, values, rows, owners, np.array([.5]))
        endpoint = forehead_extrema_prediction(2, values, rows, owners, np.array([1.]))
        self.assertEqual(midpoint[0], .5)
        self.assertEqual(endpoint[0], 0.)
        self.assertNotEqual(midpoint[0], c[0]+.5*(endpoint[0]-c[0]))

    def test_SOC_expands_all_children_and_uses_alpha_parent_prediction(self):
        calls = []
        bases = []
        def constraints(x):
            return np.array([min(x[0]-.6, .4-x[0]), .001+x[1]-x[0]**2])
        def children(x):
            bases.append(x.copy())
            return model([x[0]-.6, .4-x[0]], [[1., 0.], [-1., 0.]])
        def lp(*args, **kwargs):
            calls.append(kwargs)
            return OptimizeResult(success=True, message='synthetic direction',
                x=np.array([1., 0., 1.]) if len(calls) == 1 else np.array([0., .2495, .2495]))
        with patch.object(solver, 'linprog', side_effect=lp):
            result, metadata = self.solve(start=np.zeros(2), radii=np.ones(2),
                constraints=constraints,
                jacobian=lambda x: sparse.csr_matrix([[1., 0.], [-2*x[0], 1.]]),
                admit=lambda x: False, linear_model=children, max_iterations=1,
                soc_source_domain=lambda x: True, soc_orientation_floor=np.array([.0005]))
        self.assertEqual(len(calls), 2)
        np.testing.assert_array_equal(bases, [[0, 0], [.5, 0]])
        # SOC parent goal is -.1, so both tied child correction RHSs are zero.
        np.testing.assert_allclose(calls[1]['b_ub'][1:3], [0, 0], atol=1e-15)
        np.testing.assert_array_equal(calls[1]['A_ub'].toarray()[1:3, :2], [[-1, 0], [1, 0]])
        self.assertFalse(result.success)
        self.assertEqual(metadata['steps'][0]['merit_before'], .6)
        self.assertGreaterEqual(constraints(result.x)[1], .0005-1e-15)

    def test_nonlinear_domain_and_saved_admission_remain_final(self):
        child = lambda x: model([x[0]-.5, 1.], [[1.], [0.]])
        result, metadata = self.solve(linear_model=child, admit=lambda x: False,
            domain=lambda x: x[0] < .4)
        self.assertFalse(result.success)
        self.assertLess(result.x[0], .4)
        self.assertTrue(any('Full source' in t.get('rejected', '')
            for row in metadata['steps'] for t in row['trials']))
        result, metadata = self.solve(linear_model=child, admit=lambda x: False)
        self.assertFalse(result.success)
        self.assertFalse(metadata['full_model_success'])

    def test_model_time_is_inside_original_deadline(self):
        now = [0.]
        def child(x):
            now[0] = 6.
            return model([x[0]-.5, 1.], [[1.], [0.]])
        with patch.object(solver.time, 'monotonic', side_effect=lambda: now[0]), \
                patch.object(solver, 'linprog', side_effect=AssertionError('expired LP')):
            result, metadata = self.solve(linear_model=child)
        self.assertEqual(metadata['status'], 'wall_limit')
        self.assertEqual(result.nit, 0)

    def test_rounded_camera_frame_original_domain_supported(self):
        p, proposal, config = fixture()
        config['frames']['right45'] = [[.70710678, .70710678, 0], [0, 0, 1]]
        stage = ForeheadCompleteGuideFairingStage(proposal, p, 'rounded-frame', config)
        differential = np.zeros((len(p), 3, 2))
        differential[:, 0, 0] = differential[:, 2, 1] = 1
        m = forehead_role_extrema_model(stage, p, np.arange(len(p)), p, differential,
            np.zeros(len(p), bool), 0, np.ones(stage.count), stage.constraints(p))
        self.assertIn(8, m.handled_rows)

    def test_tied_fixed_winner_does_not_reduce_existing_FD_margin(self):
        p, proposal, config = fixture()
        config['max_evaluations'] = 10000
        p[:6] = np.r_[np.zeros((1, 3)), np.cumsum(np.array(
            [[2, 1, 1], [1, 2, 1], [1, 1, 2], [2, 1, 1], [1, 2, 1]])/1024, axis=0)]
        stage = ForeheadCompleteGuideFairingStage(proposal, p, 'tie-margin', config)
        ids = np.array([2])
        differential = np.array([[[1., 0.], [0., 0.], [0., 1.]]])
        c, selected, flags, deps, _ = forehead_cd_parameter_constraints(
            stage, p, ids, differential, np.array([False]))
        x = p[ids][:, [0, 2]].ravel()*1000
        def evaluate(values):
            q = p.copy()
            q[ids, 0], q[ids, 2] = values[0]/1000, values[1]/1000
            return (q,)
        old = forehead_complete_fallback_block(stage, x, evaluate, selected, flags, deps)
        before = old.copy()
        m = forehead_role_extrema_model(stage, p, ids, p[ids], differential,
            np.array([False]), 0, np.ones(stage.count), c)
        forehead_extrema_rows(c, old, m)
        targets, _ = solver.forehead_quantization_margin_targets(c, old, np.ones(stage.count),
            np.array([], int), np.array([], int), [], [])
        np.testing.assert_array_equal(old, before)
        np.testing.assert_array_equal(selected[3], [0, 0])
        self.assertEqual(targets[3], 1e-7)

    def test_disabled_factory_and_shared_budget_not_swallowed(self):
        self.assertIsNone(forehead_make_extrema_model(None, None, None, None, None, None))
        def exhausted(x):
            raise ValueError('Complete-guide shared evaluation budget exhausted')
        with self.assertRaisesRegex(ValueError, 'budget exhausted'):
            self.solve(linear_model=exhausted)


if __name__ == '__main__':
    unittest.main()
