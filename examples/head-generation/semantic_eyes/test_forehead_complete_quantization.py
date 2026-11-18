"""Observed float32 drift requests stronger targets, never saved admission."""
import copy
import unittest
from unittest.mock import patch
import numpy as np
from scipy.sparse import csr_matrix
from forehead_complete_guide_stage import ForeheadCompleteGuideFairingStage
from forehead_quantization_targets import (forehead_complete_margin_targets,
                                           forehead_make_quantization_margins)
from test_forehead_complete_guide_stage import fixture


def straddled_fixture(scale=1.):
    points, proposal, config = fixture()
    points = (points * scale).astype(np.float32).astype(float)
    proposal['vertices'] = points.copy()
    initial = ForeheadCompleteGuideFairingStage(proposal, points, 'source', config)
    continuous = points.copy()
    continuous[2, 1] += float(np.spacing(np.float32(points[2, 1]))) / 4
    smooth = initial.measure(continuous)
    saved = initial.measure(continuous.astype(np.float32).astype(float))
    candidates = np.flatnonzero(saved > smooth)
    if not len(candidates):
        raise AssertionError('Synthetic fixture must straddle float32 admission')
    row = candidates[np.argmax((saved[candidates] - smooth[candidates]) /
                              np.maximum(abs(saved[candidates]), 1.))]
    caps = np.maximum(saved, smooth)
    caps[row] = (saved[row] + smooth[row]) / 2
    proposal['complete_guide_fairing_guard'] = {
        'version': 1, 'identity': initial.identity, 'source_identity': 'source',
        'config': config, 'reference': initial.reference, 'caps': caps.tolist(),
        'energy_cap': max(initial.energy_from(smooth), initial.energy_from(saved)) + 1.}
    return continuous, proposal, ForeheadCompleteGuideFairingStage(proposal, points, 'source', None)


class CompleteQuantizationTests(unittest.TestCase):
    def test_owned_failed_rows_only_and_energy_row(self):
        smooth = np.array([.2, -.1, 0., .5])
        saved = np.array([-.03, .01, 0., -.2])
        for scale in (1e-12, 1., 1e12):
            scales = np.full(9, scale)
            target = forehead_complete_margin_targets(smooth, saved, 2, scales)
            np.testing.assert_array_equal(np.flatnonzero(target), [2, 5])
            self.assertTrue(np.all(target[[2, 5]] > smooth[[0, 3]] / scale))
            self.assertTrue(np.all(target[[2, 5]] >= (smooth-saved)[[0, 3]] / scale))
            np.testing.assert_array_equal(smooth, [.2, -.1, 0., .5])
            np.testing.assert_array_equal(saved, [-.03, .01, 0., -.2])

    def test_zero_boundary_and_no_drift_do_not_request_clearance(self):
        for value in (0., 1e-300, 1., 1e300):
            np.testing.assert_array_equal(
                forehead_complete_margin_targets([value], [value], 0, [1.]), [0.])
        target = forehead_complete_margin_targets([0.], [-np.nextafter(0., 1.)], 0, [1.])
        self.assertGreater(target[0], 0.)

    def test_large_current_residual_still_gets_strictly_stronger_target(self):
        target = forehead_complete_margin_targets([1e300], [-1e-300], 0, [1.])
        self.assertTrue(np.isfinite(target).all())
        self.assertGreater(target[0], 1e300)
        with self.assertRaisesRegex(ValueError, 'Unrepresentable'):
            forehead_complete_margin_targets([1e308], [-1e308], 0, [1e-300])

    def test_extended_precision_clearance_cannot_overflow_return_vector(self):
        if np.finfo(np.longdouble).max <= np.finfo(float).max:
            self.skipTest('Platform has no wider floating point type')
        safety = np.longdouble('1e400')
        self.assertTrue(np.isfinite(safety))
        with self.assertRaisesRegex(ValueError, 'Unrepresentable'):
            forehead_complete_margin_targets([1.], [-1.], 0, [1.], safety)
        finite = forehead_complete_margin_targets([1.], [-1.], 0, [1.], np.longdouble('1e100'))
        self.assertEqual(finite.dtype, np.dtype('float64'))
        self.assertTrue(np.isfinite(finite).all())

    def test_invalid_ownership_scaling_and_finite_data_rejected(self):
        bad = [([1.], [0., 1.], 0, [1.]), ([1.], [-1.], True, [1.]),
               ([1.], [-1.], -1, [1.]), ([1.], [-1.], 1, [1.]),
               ([1.], [-1.], 0, [0.]), ([1.], [-1.], 0, [np.inf]),
               ([np.nan], [-1.], 0, [1.]), ([1.], [np.inf], 0, [1.])]
        for args in bad:
            with self.subTest(args=args), self.assertRaises(ValueError):
                forehead_complete_margin_targets(*args)
        for safety in (0., -1., np.nan, np.inf, True, 'unsafe', 1j):
            with self.subTest(safety=safety), self.assertRaises(ValueError):
                forehead_complete_margin_targets([1.], [-1.], 0, [1.], safety)

    def test_actual_float32_failure_is_not_mistaken_for_admission(self):
        for scale in (1e-4, 1., 1e4):
            with self.subTest(scale=scale):
                points, proposal, stage = straddled_fixture(scale)
                saved = points.astype(np.float32).astype(float)
                self.assertTrue(stage.accepts(points))
                self.assertFalse(stage.accepts(saved))
                immutable = copy.deepcopy((stage.caps, stage.reference, stage.target, stage.config))
                before = stage.total_calls
                targets = stage.quantization_targets(points, 3, np.ones(stage.count + 6))
                self.assertEqual(stage.total_calls - before, 3)
                self.assertEqual(stage.calls['quantization_target_request'], 1)
                failed = stage.constraints(saved) < 0
                self.assertTrue(failed.any())
                self.assertTrue(np.all(targets[3:3+stage.count][failed] > stage.constraints(points)[failed]))
                np.testing.assert_array_equal(stage.caps, immutable[0])
                self.assertEqual((stage.reference, stage.target, stage.config), immutable[1:])
                self.assertFalse(stage.accepts(saved))
                with self.assertRaisesRegex(ValueError, 'unadmitted'):
                    stage.capture(proposal, points)

    def test_request_and_both_measurements_cannot_exceed_budget(self):
        for available in (0, 1, 2):
            points, _, stage = straddled_fixture()
            stage.config['max_evaluations'] = stage.total_calls + available
            with self.assertRaisesRegex(ValueError, 'budget exhausted'):
                stage.quantization_targets(points, 0, np.ones(stage.count))
            self.assertEqual(stage.total_calls, stage.config['max_evaluations'])

    def test_energy_row_uses_same_saved_drift_policy(self):
        points, _, stage = straddled_fixture()
        smooth = np.zeros(stage.count)
        saved = smooth.copy()
        smooth[-1], saved[-1] = 1e-8, -2e-8
        with patch.object(stage, 'constraints', side_effect=[smooth, saved]):
            target = stage.quantization_targets(points, 2, np.ones(stage.count + 4))
        np.testing.assert_array_equal(np.flatnonzero(target), [stage.count + 1])
        self.assertGreater(target[-3], 3e-8)

    def test_generated_flattened_margins_keep_original_saved_gate(self):
        from source_nasal_spacing import adapt
        pipeline = """def pipeline():
        report.update(refine_nasal_source(data))
        from shell import refine_chin_source
        report.update(refine_chin_source(data,report["chin_source_anchors"]))
"""
        emitted = adapt({'pipeline.py': pipeline, 'shell.py': ''})
        namespace = {}
        exec(compile(emitted['shell.py'], 'generated-complete-quantization', 'exec'), namespace)
        points, proposal, _ = straddled_fixture()
        stage = namespace['ForeheadCompleteGuideFairingStage'](
            proposal, proposal['vertices'], 'source', None)
        self.assertFalse(stage.accepts(points.astype(np.float32).astype(float)))
        target = stage.quantization_targets(points, 0, np.ones(stage.count))
        self.assertTrue(np.any(target > 0))
        self.assertFalse(stage.accepts(points.astype(np.float32).astype(float)))
        self.assertTrue(callable(namespace['forehead_make_quantization_margins']))

    def test_factory_preserves_prior_targets_and_disabled_behavior(self):
        from types import SimpleNamespace
        points = np.array([[0., 0, 0], [1., 0, 0], [1., 1, 0], [0., 1, 0]])
        prior = np.array([2., 0., 0., 0.])
        added = np.array([0., 0., 3., 0.])
        stage = SimpleNamespace(quantization_targets=lambda *args: added.copy())
        layout = {'angle_lower': (0, 1), 'angle_upper': (1, 2),
                  'orientation': (2, 3), 'complete_guide_fairing': (2, 4)}
        args = (lambda x: (points, np.array([[90.]*4])), lambda x: csr_matrix(np.eye(4)),
                layout, np.ones(4), np.array([[0, 1, 2, 3]]), np.ones((1, 4)), 1., np.ones(4))
        with patch('forehead_quantization_targets.forehead_quantization_margin_targets', return_value=(prior.copy(), {})), patch(
                'forehead_quantization_targets.forehead_orientation_margin_targets', side_effect=lambda t, *args, **kw: (t, [])):
            old, old_evidence = forehead_make_quantization_margins(*args)(np.zeros(4), np.zeros(4))
            new, evidence = forehead_make_quantization_margins(*args, complete_stage=stage)(np.zeros(4), np.zeros(4))
        np.testing.assert_array_equal(old, prior)
        np.testing.assert_array_equal(new, np.maximum(prior, added))
        self.assertNotIn('complete_guide_quantization', old_evidence)
        self.assertEqual(evidence['complete_guide_quantization']['rows'], [2])


if __name__ == '__main__':
    unittest.main()
