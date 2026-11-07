"""Guarded derivatives of tiny synthetic functions; never a model fit."""

from contextlib import ExitStack
from types import SimpleNamespace
import copy
import unittest
from unittest.mock import patch

import numpy as np
from scipy.optimize._numdiff import approx_derivative
from scipy.sparse import csr_matrix

import forehead_local_shape_operator as operator
from forehead_operator_selected_shape_samples import ForeheadSelectedForwardSamples
from forehead_operator_shape_analytic_samples import ForeheadAnalyticShapeSamples
from forehead_surface_batch import ForeheadSurfaceBatch
from local_shape_safe_derivative import local_shape_safe_derivative


def forbidden(*args, **kwargs):
    raise AssertionError('Unused derivative path executed')


class NumericalProbe:
    def __init__(self):
        self.calls = []

    def __call__(self, function, values, **kwargs):
        self.calls.append((values.copy(), kwargs))
        if kwargs != {'method': '2-point', 'abs_step': 1e-5}:
            raise AssertionError('Changed numerical convention')
        initial = function(values)
        result = np.empty((len(initial), len(values)))
        for column in range(len(values)):
            probe = values.copy()
            probe[column] += 1e-5
            result[:, column] = (function(probe) - initial) / 1e-5
        return result


class Assembly:
    count = 1

    def __init__(self, opened=False):
        self.opened, self.calls = opened, []

    def jacobian(self, points, field, ids, geometry, expression):
        self.calls.append((np.array(ids), geometry.copy(), expression.copy()))
        if self.opened:
            derivative = geometry + expression
            row = np.r_[2000 * derivative[0, 1], 5000 * derivative[1, 1]]
        else:
            row = np.r_[1000 * geometry[0, 0], 3000 * geometry[1, 0]]
        return csr_matrix(row[None, :])


class LinearProblem:
    expected = np.array([[1., 2., 0., 0.], [0., 0., 1., -1.],
                         [1., 0., 3., 0.], [0., 2.2, 0., 5.5]])

    def __init__(self, geometry_flags=(False, False), field_flags=(False, False)):
        self.values, self.ids = np.array([1., 2., 3., 4.]), [1, 2]
        self.flags = [np.array(geometry_flags), np.array(field_flags)]
        self.missing = [False, False]
        self.none_jacobian = [False, False]
        self.jacobians = np.tile([[1., 0.], [0., 1.], [0., 0.]], (2, 1, 1))
        self.invalid = lambda values: False
        self.forward, self.reverse = Assembly(), Assembly(True)
        self.numerical = NumericalProbe()
        self.batch = SimpleNamespace(sample_differential=lambda xy: self.differential(xy, 0))
        self.field_batch = SimpleNamespace(sample_differential=lambda xy: self.differential(xy, 1))

    def source(self, coordinates):
        points = np.zeros((3, 3))
        points[1:, :2] = coordinates
        points[1:, 2] = [1., 2.]
        return points, .1 * points

    def evaluate(self, values):
        return None if self.invalid(values) else self.source(values.reshape(-1, 2) / 1000)

    def differential(self, coordinates, expression):
        if self.missing[expression]:
            return None
        primal = self.source(coordinates)[expression][1:]
        derivative = None if self.none_jacobian[expression] else self.jacobians * (.1 if expression else 1)
        return primal, derivative, self.flags[expression]

    def base(self, values):
        return self.expected[:2] @ values

    def constraints(self, values):
        return np.full(4, -1000.) if self.invalid(values) else self.expected @ values

    def run(self):
        return local_shape_safe_derivative(
            self.values, self.evaluate, self.base, self.constraints, self.batch,
            self.field_batch, self.ids, self.forward, self.reverse, 2, self.numerical)


class SafeLocalDerivativeTests(unittest.TestCase):
    def test_smooth_mixed_and_all_boundary_match_independent_linear_derivative(self):
        for geometry, field in (((False, False), (False, False)),
                                ((True, False), (False, False)),
                                ((False, False), (False, True)),
                                ((True, False), (False, True))):
            problem = LinearProblem(geometry, field)
            np.testing.assert_allclose(problem.run(), problem.expected, rtol=0, atol=2e-9)

    def test_actual_scipy_two_point_path_matches_linear_derivative(self):
        for flags in ((False, False), (True, False), (True, True)):
            problem = LinearProblem(flags)
            problem.values = np.array([.123, -2.345, 3.987, -4.123])
            problem.numerical = approx_derivative
            np.testing.assert_allclose(problem.run(), problem.expected, rtol=0, atol=2e-9)

    def test_whole_geometry_or_field_none_uses_full_numerical_fallback(self):
        for missing in range(2):
            problem = LinearProblem()
            problem.missing[missing] = True
            np.testing.assert_allclose(problem.run(), problem.expected, rtol=0, atol=2e-9)
            self.assertFalse(problem.forward.calls or problem.reverse.calls)
            self.assertEqual(problem.numerical.calls[0][0].shape, (4,))

    def test_none_jacobian_inside_valid_tuple_also_uses_full_fallback(self):
        for missing in range(2):
            problem = LinearProblem()
            problem.none_jacobian[missing] = True
            np.testing.assert_allclose(problem.run(), problem.expected, rtol=0, atol=2e-9)
            self.assertFalse(problem.forward.calls or problem.reverse.calls)

    def test_missing_geometry_does_not_evaluate_field_differential(self):
        problem = LinearProblem()
        problem.missing[0] = True
        problem.field_batch.sample_differential = forbidden
        self.assertTrue(np.isfinite(problem.run()).all())

    def test_invalid_basepoint_never_becomes_a_flat_sentinel_derivative(self):
        problem = LinearProblem()
        problem.invalid = lambda values: True
        with self.assertRaisesRegex(ValueError, 'source probe'):
            problem.run()
        self.assertFalse(problem.numerical.calls)

    def test_invalid_full_base_and_central_probes_are_rejected(self):
        for mode in ('full', 'base', 'central'):
            problem = LinearProblem((mode == 'central', False))
            problem.missing[0] = mode == 'full'
            problem.invalid = lambda values: values[0] > problem.values[0]
            with self.subTest(mode=mode), self.assertRaisesRegex(ValueError, 'source probe'):
                problem.run()

    def test_invalid_negative_central_probe_is_rejected(self):
        problem = LinearProblem((True, False))
        problem.invalid = lambda values: values[0] < problem.values[0]
        with self.assertRaisesRegex(ValueError, 'source probe'):
            problem.run()

    def test_finite_negative_residual_including_minus_1000_is_valid(self):
        problem = LinearProblem()
        problem.missing[0] = True
        problem.constraints = lambda values: np.full(4, -1000.)
        np.testing.assert_array_equal(problem.run(), np.zeros((4, 4)))

    def test_all_boundary_skips_base_fd_and_both_analytic_assemblies(self):
        problem = LinearProblem((True, False), (False, True))
        problem.jacobians[:] = np.nan
        problem.base = problem.numerical = forbidden
        problem.forward.jacobian = problem.reverse.jacobian = forbidden
        np.testing.assert_allclose(problem.run(), problem.expected, rtol=0, atol=2e-9)

    def test_mixed_boundary_nan_is_zeroed_in_copies_only(self):
        problem = LinearProblem((True, False))
        problem.jacobians[0] = np.nan
        np.testing.assert_allclose(problem.run(), problem.expected, rtol=0, atol=2e-9)
        for assembly in (problem.forward, problem.reverse):
            ids, geometry, field = assembly.calls[-1]
            np.testing.assert_array_equal(ids, [1, 2])
            np.testing.assert_array_equal(geometry[0], np.zeros((3, 2)))
            np.testing.assert_array_equal(field[0], np.zeros((3, 2)))
        self.assertTrue(np.isnan(problem.jacobians[0]).all())
        self.assertEqual(problem.numerical.calls[0][0].shape, (2,))

    def test_unused_one_sided_boundary_base_probe_is_not_attempted(self):
        problem = LinearProblem((True, False))
        problem.invalid = lambda values: 0 < values[0] - problem.values[0] < 5e-5
        self.assertTrue(np.isfinite(problem.run()).all())

    def test_consumed_nonfinite_jacobian_rejects(self):
        problem = LinearProblem()
        problem.jacobians[0] = np.nan
        with self.assertRaisesRegex(ValueError, 'Nonfinite consumed'):
            problem.run()

    def test_malformed_parameters_and_source_states_reject(self):
        for values in (np.ones((2, 2)), np.ones(3), np.array([1., 2., 3., np.nan])):
            problem = LinearProblem()
            problem.values = values
            with self.assertRaisesRegex(ValueError, 'parameters'):
                problem.run()
        for state in (None, [np.zeros((3, 3))], (np.zeros(3), np.zeros(3)),
                      (np.zeros((3, 3)), np.zeros((2, 3)))):
            problem = LinearProblem()
            problem.evaluate = lambda values: state
            with self.assertRaises(ValueError):
                problem.run()

    def test_nonfinite_closed_field_or_open_pose_rejects(self):
        for part in (0, 1, 2):
            problem = LinearProblem()
            points, field = problem.source(problem.values.reshape(-1, 2) / 1000)
            if part == 2:
                points[:] = field[:] = np.finfo(float).max
            else:
                (points, field)[part][0, 0] = np.nan
            problem.evaluate = lambda values: (points, field)
            with np.errstate(over='ignore'), self.assertRaisesRegex(ValueError, 'Nonfinite'):
                problem.run()

    def test_bad_full_and_base_residual_shapes_or_values_reject(self):
        for method in ('constraints', 'base'):
            for result in (np.zeros(1), np.array([np.nan] * (4 if method == 'constraints' else 2))):
                problem = LinearProblem()
                setattr(problem, method, lambda values: result)
                with self.assertRaisesRegex(ValueError, 'probe residual'):
                    problem.run()

    def test_malformed_differentials_and_primal_mismatch_reject(self):
        problem = LinearProblem()
        primal, derivative, flags = problem.differential(problem.values.reshape(-1, 2) / 1000, 0)
        cases = [(primal, derivative), (primal + 1, derivative, flags),
                 (primal, derivative, flags.astype(int)), (primal, derivative, flags[:1]),
                 (primal, derivative[:, :, :1], flags)]
        for result in cases:
            problem = LinearProblem()
            problem.batch.sample_differential = lambda coordinates: result
            with self.assertRaises(ValueError):
                problem.run()

    def test_bad_numerical_derivative_shape_or_values_reject(self):
        for full in (False, True):
            for wrong_shape in (False, True):
                problem = LinearProblem()
                problem.missing[0] = full
                count = 4 if full else 2
                result = np.zeros((1, 4)) if wrong_shape else np.full((count, 4), np.nan)
                problem.numerical = lambda *args, **kwargs: result
                with self.assertRaises(ValueError):
                    problem.run()

    def test_analytic_exceptions_propagate_without_numerical_recovery(self):
        for role in ('forward', 'reverse'):
            for exception in (ValueError, RuntimeError):
                problem = LinearProblem()
                error = exception('Synthetic analytic failure')
                with patch.object(getattr(problem, role), 'jacobian', side_effect=error):
                    with self.assertRaises(exception) as raised:
                        problem.run()
                self.assertIs(raised.exception, error)
                self.assertEqual(len(problem.numerical.calls), 1)

    def test_bad_analytic_shapes_or_consumed_values_reject(self):
        for result in (csr_matrix((2, 4)), csr_matrix(np.full((1, 4), np.nan))):
            problem = LinearProblem()
            problem.forward.jacobian = lambda *args: result
            with self.assertRaises(ValueError):
                problem.run()


class RealShapeAssemblyTests(unittest.TestCase):
    def assembler(self, cls=ForeheadAnalyticShapeSamples):
        instance = object.__new__(cls)
        instance.quads, instance.face_ids = np.array([[0, 1, 2, 3]]), np.array([0])
        instance.weights, instance.active = np.array([[.1, .2, .3, .4]]), np.array([0, 1])
        instance.values = lambda *args: (np.zeros(2), [np.array([[1., 2., 3.]]),
                                                      np.array([[4., 5., 6.]])])
        return instance

    def test_empty_corner_indices_work_in_real_and_selected_assemblers(self):
        args = (np.zeros((4, 3)), np.zeros((4, 3)), [1, 2],
                np.ones((2, 3, 2)), np.ones((2, 3, 2)))
        for cls in (ForeheadAnalyticShapeSamples, ForeheadSelectedForwardSamples):
            actual = self.assembler(cls).jacobian(*args).toarray()
            np.testing.assert_allclose(actual, [[1.2, 1.2, 1.8, 1.8], [6., 6., 9., 9.]],
                                       rtol=0, atol=2e-15)

    def test_empty_variable_set_yields_zero_width_sparse_matrix(self):
        result = self.assembler().jacobian(np.zeros((4, 3)), np.zeros((4, 3)), [],
                                            np.empty((0, 3, 2)), np.empty((0, 3, 2)))
        self.assertEqual(result.shape, (2, 0))
        self.assertEqual(result.nnz, 0)

    def test_nonempty_assembly_matches_independent_weighted_gradient(self):
        geometry = np.arange(24, dtype=float).reshape(4, 3, 2)
        field = geometry / 10
        actual = self.assembler().jacobian(np.zeros((4, 3)), np.zeros((4, 3)),
                                            list(range(4)), geometry, field).toarray()
        expected = np.vstack([np.concatenate([weight * gradient @ jacobian[i]
                             for i, weight in enumerate([.1, .2, .3, .4])])
                              for gradient, jacobian in ((np.array([1., 2., 3.]), geometry),
                                                        (np.array([4., 5., 6.]), geometry + field))])
        np.testing.assert_allclose(actual, expected, rtol=3e-16, atol=2e-14)


class RealPairedSamplerTests(unittest.TestCase):
    def arguments(self, x):
        chart = np.array([[0., 0., 0.], [1., 0., 0.], [2., 0., 0.],
                          [0., 0., 1.], [1., 0., 1.], [2., 0., 1.]])
        geometry, field = chart.copy(), np.zeros_like(chart)
        geometry[[2, 5], 1], field[[2, 5], 1] = .5, .2
        quads = [[0, 1, 4, 3], [1, 2, 5, 4]]
        batch, field_batch = (ForeheadSurfaceBatch(chart, data, quads) for data in (geometry, field))
        values = np.array([1000 * x, 500.])

        def evaluate(parameters):
            coordinates = parameters.reshape(-1, 2) / 1000
            points, delta = batch.sample(coordinates), field_batch.sample(coordinates)
            return None if points is None or delta is None else (points, delta)

        def constraints(parameters):
            state = evaluate(parameters)
            if state is None:
                return np.full(4, -1000.)
            points, delta = state
            return np.array([points[0, 0], points[0, 1], delta[0, 1], (points + delta)[0, 1]])

        return (values, evaluate, forbidden, constraints, batch, field_batch, [0],
                SimpleNamespace(count=1, jacobian=forbidden),
                SimpleNamespace(count=2, jacobian=forbidden), 1, forbidden)

    def test_shared_boundary_central_surrogate_preserves_both_poses(self):
        args = self.arguments(1.)
        before = args[1](args[0])
        actual = local_shape_safe_derivative(*args)
        np.testing.assert_allclose(actual[:, 0], [.001, .00025, .0001, .00035], rtol=1e-8)
        np.testing.assert_array_equal(actual[:, 1], np.zeros(4))
        for expected, result in zip(before, args[1](args[0])):
            np.testing.assert_array_equal(result, expected)

    def test_outer_boundary_invalid_probe_does_not_use_finite_sentinel(self):
        with self.assertRaisesRegex(ValueError, 'source probe'):
            local_shape_safe_derivative(*self.arguments(0.))


class LocalOperatorWiringTests(unittest.TestCase):
    def run_operator(self, mode):
        world = np.array([[0., -1., 0.], [1., -1., 0.], [1., -1., 1.], [0., -1., 1.]])
        chart = world.copy()
        chart[:, 1] = 0
        source = {'vertices': world.tolist(), 'open_delta': np.zeros((4, 3)).tolist(),
                  'faces': [[0, 1, 2, 3]]}
        proposal = dict(copy.deepcopy(source), source_chart=chart.tolist(),
                        source_faces=[[0, 1, 2, 3]], support=[1, 2], affected_new=[0],
                        affected_old=[0], topology={'lineage': {}}, lineage={})
        before, samplers, matrices = copy.deepcopy((source, proposal)), [], []

        def sampler(chart, payload, quads):
            expression = bool(samplers)

            def sample(coordinates):
                if mode == 'invalid' and coordinates[0, 0] > 1.:
                    return None
                points = np.column_stack((coordinates[:, 0], -np.ones(len(coordinates)), coordinates[:, 1]))
                return np.zeros_like(points) if expression else points

            def differential(coordinates):
                if mode == ('field' if expression else 'geometry') or mode == 'invalid':
                    return None
                jacobian = np.tile([[1., 0.], [0., 0.], [0., 1.]], (len(coordinates), 1, 1))
                return sample(coordinates), jacobian * (0 if expression else 1), np.full(len(coordinates), mode == 'boundary')

            result = SimpleNamespace(sample=sample, sample_differential=differential)
            samplers.append(result)
            return result

        def assembly(*args, **kwargs):
            return SimpleNamespace(count=1, constraints=lambda points, field: np.array([points[:, 0].sum()]),
                                   jacobian=forbidden if mode == 'boundary' else
                                   lambda points, field, ids, geometry, expression: csr_matrix(geometry[:, 0].reshape(1, -1)))

        def no_optimization(objective, values, **kwargs):
            self.assertEqual(kwargs['method'], 'SLSQP')
            matrices.append(kwargs['constraints']['jac'](values))
            return SimpleNamespace(x=values.copy(), message='Synthetic derivative wiring only')

        replacements = dict(ForeheadSurfaceBatch=sampler, ForeheadSelectedForwardSamples=assembly,
                            ForeheadReverseShapeSamples=assembly, AuthorQuadSurface=lambda *args: SimpleNamespace(),
                            forehead_support_parameters=lambda proposal: chart.copy(),
                            forehead_support_held_nodes=lambda proposal: {0, 3},
                            forehead_support_rows=lambda value: {},
                            forehead_frozen_field=lambda values, count, rows: np.zeros((count, 3)),
                            orbital_quad_quality=lambda points, quads: (np.full((1, 4), 90.), np.ones(1)),
                            approx_derivative=NumericalProbe(), minimize=no_optimization)
        with ExitStack() as stack:
            for name, replacement in replacements.items():
                stack.enter_context(patch.object(operator, name, replacement))
            result = operator.forehead_condition_local_shape(
                source, proposal, {'target_face_ids': [0], 'source_face_ids': [0]}, {}, lambda *args: None)
        self.assertEqual((source, proposal), before)
        self.assertEqual(len(matrices), 1)
        self.assertEqual(matrices[0].shape, (17, 4))
        self.assertTrue(np.isfinite(matrices[0]).all())
        self.assertFalse(result['local_admitted'] or result['fit_complete'])
        self.assertNotIn('proposal', result)
        return result

    def test_real_operator_handles_missing_whole_differentials(self):
        for mode in ('geometry', 'field'):
            self.assertEqual(self.run_operator(mode)['variable_ids'], [1, 2])

    def test_real_operator_smooth_path_remains_unadmitted_without_scalar_guard(self):
        self.assertEqual(self.run_operator('smooth')['parameters_mm'], [1000., 0., 1000., 1000.])

    def test_real_operator_all_boundary_skips_analytic_assembly(self):
        self.run_operator('boundary')

    def test_real_operator_invalid_probe_propagates(self):
        with self.assertRaisesRegex(ValueError, 'source probe'):
            self.run_operator('invalid')


if __name__ == '__main__':
    unittest.main()
