"""Nonzero provenance survives numerical pruning on synthetic straight rails."""

from types import SimpleNamespace
import unittest
from unittest.mock import patch

import numpy as np
from scipy.sparse import csr_matrix

import forehead_hybrid_gap_jacobian as hybrid
from forehead_matched_gap_jacobian import forehead_matched_gap_jacobian
from forehead_vertex_gap_jacobian import forehead_vertex_gap_row
from target_boundary_dependency import target_has_boundary_dependency


def matched_row(points, hits, parameters, ids, boundary, world_jacobian=None):
    detail = {'world_hits': [[hits[0]], [hits[1]]],
              'curve_parameters': [[parameters[0]], [parameters[1]]]}
    fast = SimpleNamespace(
        specs=[('left', 'right', None, None, np.array([[1., 1.]]))],
        evaluate=lambda *args, **kwargs: {'valid': True, 'details': [detail]})
    stage = SimpleNamespace(fast=fast, paths={'left': [0, 1], 'right': [2, 3]},
                            targets={'left-right': 2000.}, vertex_records=lambda *args: [])
    if world_jacobian is None:
        world_jacobian = np.array([[[0., 0.], [.001, 0.], [0., 0.]]])
    return forehead_matched_gap_jacobian(
        stage, points, points[:, :2], ids, world_jacobian, np.array(boundary))


def source_row(weight, boundary=True, factor=1.):
    points = np.array([[0., 0., 0.], [0., 1., 0.], [1., -1., 0.], [1., 2., 0.]])
    derivative = np.array([[[.001 * factor, 0.], [0., .001], [0., 0.]]])
    return matched_row(points, ([0., 1 - weight, 0.], [1., .5, 0.]),
                       (1 - weight, .5), [0], [boundary], derivative)


def target_row(fraction, boundary=True):
    points = np.array([[fraction, .5, 0.], [fraction, 1.5, 0.],
                       [0., 0., 0.], [1., 0., 0.]])
    return matched_row(points, ([fraction, 1., 0.], [.5, 0., 0.]),
                       (.5, .5), [3], [boundary])


def vertex_row(fraction, node=1, boundary=True, variable=True):
    points = np.array([[0., 0., 0.], [1., 0., 0.], [fraction, 1., 0.]])
    record = {'target_mm': 2000., 'domain_boundary': False, 'active': True,
              'source_id': 2, 'target': 'rail'}
    index = {node: 0} if variable else {}
    derivative = np.array([[[0., 0.], [.001, 0.], [0., 0.]]])
    return forehead_vertex_gap_row(
        record, SimpleNamespace(paths={'rail': [0, 1]}), points, index,
        derivative if variable else np.empty((0, 3, 2)),
        np.array([boundary]) if variable else np.empty(0, bool))


class TinySourceDependencyTests(unittest.TestCase):
    def test_nonzero_source_weights_below_pruning_scale_still_flag(self):
        for weight in (1e-10, 2**-40, 2**-52):
            flagged, smooth = source_row(weight), source_row(weight, False)
            self.assertNotEqual(flagged[0][0, 0], 0.)
            self.assertTrue(flagged[2][0])
            self.assertFalse(smooth[2][0])
            np.testing.assert_array_equal(flagged[0].toarray(), smooth[0].toarray())
            np.testing.assert_array_equal(flagged[1], smooth[1])
            self.assertEqual(flagged[3]['author_boundary'], 2)

    def test_distinct_one_sided_source_derivatives_both_require_fallback(self):
        first, second = source_row(1e-10), source_row(1e-10, factor=3.)
        self.assertNotEqual(first[0][0, 0], second[0][0, 0])
        self.assertTrue(first[2][0] and second[2][0])

    def test_exact_zero_source_dependency_is_not_a_boundary(self):
        result = source_row(0.)
        self.assertFalse(result[2][0])
        # The reverse row still depends on this variable as a target.
        self.assertTrue(result[2][1])
        self.assertEqual(result[3], {'author_boundary': 1})

    def test_smooth_source_and_existing_large_weight_keep_their_status(self):
        for weight in (1e-10, .25):
            self.assertFalse(source_row(weight, False)[2][0])
        self.assertTrue(source_row(.25)[2][0])


class TinyTargetDependencyTests(unittest.TestCase):
    def test_tiny_target_weights_flag_without_altering_pruned_analytic_rows(self):
        for fraction in (2**-31, 1e-10, 1e-9):
            flagged = vertex_row(fraction)
            smooth = vertex_row(fraction, boundary=False)
            self.assertTrue(flagged[2])
            self.assertFalse(smooth[2])
            np.testing.assert_array_equal(flagged[0], [0., 0.])
            np.testing.assert_array_equal(flagged[0], smooth[0])
            self.assertEqual(flagged[1], smooth[1])
            self.assertEqual(flagged[3].count('vertex_target_author_boundary'), 1)

    def test_first_endpoint_tiny_weight_is_kept(self):
        flagged = vertex_row(1 - 2**-31, node=0)
        self.assertTrue(flagged[2])
        self.assertIn('vertex_target_author_boundary', flagged[3])
        np.testing.assert_array_equal(flagged[0], [0., 0.])

    def test_matched_target_dependency_is_independent_of_source_flags(self):
        flagged, smooth = target_row(2**-31), target_row(2**-31, False)
        self.assertTrue(flagged[2][0])
        self.assertFalse(smooth[2][0])
        np.testing.assert_array_equal(flagged[0].toarray(), smooth[0].toarray())
        np.testing.assert_array_equal(flagged[1], smooth[1])

    def test_zero_fixed_and_nonboundary_target_do_not_gain_fallback(self):
        for options in ({'fraction': 0.}, {'fraction': 2**-31, 'boundary': False},
                        {'fraction': 2**-31, 'variable': False}):
            result = vertex_row(**options)
            self.assertFalse(result[2])
            self.assertEqual(result[3], [])

    def test_existing_large_weight_has_one_reason_and_nonzero_derivative(self):
        result = vertex_row(.25)
        self.assertTrue(result[2])
        self.assertEqual(result[3], ['vertex_target_author_boundary'])
        self.assertAlmostEqual(result[0][0], -.25 / 2000)

    def test_only_original_contenders_contribute_boundary_dependency(self):
        path, fractions = [0, 1, 2], [1 - 2**-31, 2**-31]
        self.assertFalse(target_has_boundary_dependency(path, fractions, [0], {2: 0}, [True]))
        self.assertTrue(target_has_boundary_dependency(path, fractions, [0, 1], {2: 0}, [True]))
        self.assertFalse(target_has_boundary_dependency(path, fractions, [], {2: 0}, [True]))

    def test_alternate_tiny_endpoint_survives_identical_pruned_target_bases(self):
        fraction = 2**-31
        points = np.array([[-1., 0., 0.], [0., 0., 0.],
                           [1., 0., 0.], [fraction, 1., 0.]])
        record = {'target_mm': 2000., 'domain_boundary': False, 'active': True,
                  'source_id': 3, 'target': 'rail'}
        result = forehead_vertex_gap_row(
            record, SimpleNamespace(paths={'rail': [0, 1, 2]}), points, {2: 0},
            np.array([[[0., 0.], [.001, 0.], [0., 0.]]]), np.array([True]))
        self.assertTrue(result[2])
        self.assertNotIn('distinct_vertex_target_tie', result[3])
        self.assertEqual(result[3], ['vertex_target_author_boundary'])
        np.testing.assert_array_equal(result[0], [0., 0.])

    def test_index_membership_not_parameter_bounds_controls_dependency(self):
        self.assertTrue(target_has_boundary_dependency([0, 1], [2**-31], [0], {1: 0}, [True]))
        self.assertFalse(target_has_boundary_dependency([0, 1], [2**-31], [0], {}, []))
        self.assertFalse(target_has_boundary_dependency([0, 1], [0.], [0], {1: 0}, [True]))


class TinyDependencyConsumerTests(unittest.TestCase):
    def fixture(self):
        points = np.array([[0., 0., 0.], [1., 0., 0.],
                           [2**-31, 1., 0.], [0., 1., 0.]])
        records = [{'target_mm': 2000., 'domain_boundary': False, 'active': True,
                    'source_id': source, 'target': 'rail'} for source in (2, 3)]
        fast = SimpleNamespace(specs=[], evaluate=lambda *a, **k: {'valid': True, 'details': []})
        stage = SimpleNamespace(fast=fast, paths={'rail': [0, 1]}, size=2,
                                vertex_records=lambda *args: records,
                                measure=lambda *args: np.array([1000., 1000.]),
                                constraints=lambda *args: np.array([.4, .6]))
        batch = SimpleNamespace(sample_differential=lambda *args: (
            points[[1]], np.array([[[0., 0.], [1., 0.], [0., 0.]]]), np.array([True])))
        return points, stage, batch

    def derivative(self, points, stage, batch, evaluate=None):
        return hybrid.forehead_make_hybrid_gap_jacobian(
            lambda values: np.zeros(1), evaluate or (lambda values: (points,)),
            lambda values: points[:, :2], batch, [1], stage, 1, np.array([2., 4., 8.]),
            csr_matrix(np.ones((3, 2), bool)),
            base_derivative=lambda values: csr_matrix([[13., 17.]]))

    def test_whole_flagged_row_uses_original_pattern_and_other_rows_remain(self):
        points, stage, batch = self.fixture()
        calls = []

        def numerical(function, values, **kwargs):
            calls.append(kwargs)
            np.testing.assert_array_equal(function(values), [.4])
            np.testing.assert_array_equal(kwargs['sparsity'][0].toarray(), [[True, True]])
            self.assertEqual(kwargs['method'], '2-point')
            self.assertEqual(kwargs['rel_step'], 1e-8)
            return csr_matrix([[29., 31.]])

        with patch.object(hybrid, 'forehead_gap_approx_derivative', numerical):
            derivative, stats = self.derivative(points, stage, batch)
            result = derivative(np.array([1., 2.])).toarray()
        self.assertEqual(len(calls), 1)
        np.testing.assert_array_equal(result, [[13., 17.], [29 / 4, 31 / 4], [0., 0.]])
        self.assertEqual(stats['fallback_rows_total'], 1)
        self.assertEqual(stats['analytic_rows_total'], 1)
        self.assertEqual(stats['flag_reasons'], {'vertex_target_author_boundary': 1})

    def test_invalid_complete_mode_numerical_probe_fails_closed(self):
        points, stage, batch = self.fixture()
        evaluate = lambda values: (points,) if np.array_equal(values, [1., 2.]) else None
        with patch.object(hybrid, 'forehead_gap_approx_derivative',
                          lambda function, values, **kwargs: function(values + 1)):
            derivative, _ = self.derivative(points, stage, batch, evaluate)
            with self.assertRaisesRegex(ValueError, 'Invalid source probe'):
                derivative(np.array([1., 2.]))


if __name__ == '__main__':
    unittest.main()
