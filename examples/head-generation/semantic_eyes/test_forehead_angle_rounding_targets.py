"""Saved-failing row ownership, fixed units, additive policy and generated use."""
from contextlib import ExitStack
import ast
from fractions import Fraction
from pathlib import Path
import sys
import tempfile
from types import SimpleNamespace
from unittest.mock import patch
import unittest
import numpy as np
from scipy.sparse import csr_matrix
from orbital_support_rule import orbital_quad_quality
from forehead_angle_rounding_targets import forehead_float32_angle_targets
from forehead_quantization_targets import forehead_make_quantization_margins
from test_forehead_angle_rounding_bound import SYNTHETIC_CORNER


def target_fixture(lower=False):
    if lower:
        theta = np.radians(25.002)
        points = np.array([[.1, 0, 0], [0., 0, 0],
            [.1*np.cos(theta), .1*np.sin(theta), 0],
            [.1+.1*np.cos(theta), .1*np.sin(theta), 0]])+1000.
    else:
        points = np.vstack((SYNTHETIC_CORNER, SYNTHETIC_CORNER[0]+SYNTHETIC_CORNER[2]-SYNTHETIC_CORNER[1]))
    faces = np.array([[0, 1, 2, 3]])
    smooth, _ = orbital_quad_quality(points, faces)
    saved, _ = orbital_quad_quality(points.astype(np.float32).astype(float), faces)
    return points, faces, smooth, saved


class AngleRoundingTargetTests(unittest.TestCase):
    def test_upper_and_lower_failures_own_only_original_rows(self):
        for lower in (False, True):
            points, faces, smooth, saved = target_fixture(lower)
            self.assertTrue(np.any(saved < 25.) if lower else np.any(saved > 150.))
            lo, hi = np.arange(2, 6), np.arange(7, 11)
            for factor in (1e-12, 1., 1e12):
                scales = np.arange(1., 13.)*factor
                targets, evidence = forehead_float32_angle_targets(points, faces, scales,
                    lo, hi, smooth, saved)
                failed = np.flatnonzero((saved.ravel() < 25.) | (saved.ravel() > 150.))
                expected_rows = [int(hi[k] if saved.ravel()[k] > 150. else lo[k]) for k in failed]
                self.assertEqual([row['row'] for row in evidence], expected_rows)
                self.assertEqual(np.flatnonzero(targets).tolist(), sorted(expected_rows))
                for item in evidence:
                    row = item['row']
                    buffer = Fraction.from_float(150.)-Fraction.from_float(149.997) if row in hi else (
                        Fraction.from_float(25.003)-Fraction.from_float(25.))
                    raw = max(Fraction(0), Fraction.from_float(item['bound_degrees'])+
                        Fraction.from_float(.001)-buffer)
                    self.assertGreaterEqual(Fraction.from_float(targets[row]),
                        raw/Fraction.from_float(scales[row]))
                    self.assertTrue(item['current_candidate_only'])
                self.assertFalse(np.shares_memory(targets, scales))

    def test_passing_saved_candidates_leave_all_targets_zero(self):
        points = np.array([[0., 0, 0], [1., 0, 0], [1., 1, 0], [0., 1, 0]])
        faces = np.array([[0, 1, 2, 3]])
        angles, _ = orbital_quad_quality(points, faces)
        targets, evidence = forehead_float32_angle_targets(points, faces, np.ones(8),
            np.arange(4), np.arange(4, 8), angles, angles)
        self.assertFalse(targets.any())
        self.assertEqual(evidence, [])

    def test_stale_candidate_measurements_and_ownership_rejected(self):
        p, q, smooth, saved = target_fixture()
        args = dict(points=p, quads=q, scales=np.ones(8), lower_rows=np.arange(4),
                    upper_rows=np.arange(4, 8), smooth_angles=smooth, saved_angles=saved)
        invalid = [dict(smooth_angles=smooth+.1), dict(saved_angles=saved+.1),
                   dict(scales=np.zeros(8)), dict(upper_rows=np.arange(4)),
                   dict(lower_rows=np.arange(4, dtype=float)), dict(quads=q+.5),
                   dict(quads=q+4), dict(quads=q-1), dict(safety_degrees=0.),
                   dict(scales=np.full(8, np.inf)), dict(upper_hard=20.)]
        for changed in invalid:
            with self.subTest(changed=changed), self.assertRaises(ValueError):
                forehead_float32_angle_targets(**dict(args, **changed))
        with self.assertRaisesRegex(ValueError, 'Unrepresentable'):
            forehead_float32_angle_targets(**dict(args, scales=np.full(8, np.nextafter(0., 1.))))

    def test_factory_max_composition_charge_and_disabled_behavior(self):
        p, faces, smooth, saved = target_fixture()
        prior = np.arange(13.)/100
        extra = np.zeros(13)
        extra[-1] = .75
        charges = []
        stage = SimpleNamespace(charge=charges.append, quantization_targets=lambda *a: extra)
        layout = {'angle_lower': (0, 4), 'angle_upper': (4, 8),
                  'orientation': (8, 12), 'complete_guide_fairing': (12, 13)}
        args = (lambda x: (p, smooth), lambda x: csr_matrix(np.eye(13)),
                layout, np.ones(13), faces, np.ones((1, 4)), 1., np.ones(4))
        expected, _ = forehead_float32_angle_targets(p, faces, np.ones(13),
            np.arange(4), np.arange(4, 8), smooth, saved)
        with ExitStack() as stack:
            stack.enter_context(patch('forehead_quantization_targets.forehead_quantization_margin_targets',
                return_value=(prior.copy(), {})))
            stack.enter_context(patch('forehead_quantization_targets.forehead_orientation_margin_targets',
                side_effect=lambda t, *a, **k: (t, [])))
            old, old_evidence = forehead_make_quantization_margins(*args)(np.zeros(13), np.zeros(13))
            new, evidence = forehead_make_quantization_margins(*args, complete_stage=stage)(np.zeros(13), np.zeros(13))
            np.testing.assert_array_equal(old, prior)
            np.testing.assert_array_equal(new, np.maximum.reduce([prior, expected, extra]))
            self.assertEqual(charges, ['float32_angle_clearance_request'])
            self.assertNotIn('float32_angle_clearance', old_evidence)
            self.assertTrue(evidence['float32_angle_clearance'])
            stage.charge = lambda kind: (_ for _ in ()).throw(ValueError('budget exhausted'))
            with self.assertRaisesRegex(ValueError, 'budget exhausted'):
                forehead_make_quantization_margins(*args, complete_stage=stage)(np.zeros(13), np.zeros(13))
        np.testing.assert_array_equal(orbital_quad_quality(p.astype(np.float32).astype(float), faces)[0], saved)
        self.assertTrue(np.any(saved > 150.))

    def test_generated_flattened_targets_and_bound_match_modules(self):
        from source_nasal_spacing import adapt
        from feature_flow_recipe import retained_sources
        retained = retained_sources()
        for name in ('forehead_angle_rounding_bound.py', 'forehead_angle_rounding_targets.py',
                     'forehead_quantization_targets.py'):
            self.assertEqual(retained[name], (Path(__file__).parent/name).read_text())
        pipeline = '''def pipeline():
        report.update(refine_nasal_source(data))
        from shell import refine_chin_source
        report.update(refine_chin_source(data,report["chin_source_anchors"]))
'''
        emitted = adapt({'pipeline.py': pipeline, 'shell.py': ''})
        namespace = {}
        exec(compile(emitted['shell.py'], 'generated-angle-rounding', 'exec'), namespace)
        p, q, smooth, saved = target_fixture()
        args = (p, q, np.ones(8), np.arange(4), np.arange(4, 8), smooth, saved)
        expected, evidence = forehead_float32_angle_targets(*args)
        actual, generated_evidence = namespace['forehead_float32_angle_targets'](*args)
        np.testing.assert_array_equal(actual, expected)
        self.assertEqual(generated_evidence, evidence)
        sys.path.insert(0, str(Path(__file__).resolve().parent.parent))
        from source_layout.module_layout import MODULE_STAGES, module_layout, execute_ordered_module
        from source_layout.layout import reconstructed
        flat = emitted['shell.py']
        present = {node.name for node in ast.parse(flat).body
                   if isinstance(node, (ast.FunctionDef, ast.ClassDef))}
        # Actual append payload, with inert markers for only absent earlier stages.
        earlier = 'pass\n'+''.join('def '+name+'():\n    pass\n'
            for _, name in MODULE_STAGES['shell.py'][1:] if name not in present)
        files, record = module_layout(earlier+flat, 'shell.py')
        self.assertEqual(reconstructed(files, record), earlier+flat)
        for name in ('shell_forehead_angle_rounding_bound.py',
                     'shell_forehead_angle_rounding_targets.py',
                     'shell_forehead_quantization_targets.py'):
            self.assertIn(name, files)
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            for name, source in files.items():
                (root/name).write_text(source, encoding='utf-8')
            replayed = {}
            execute_ordered_module(root/'shell.py', record, replayed)
            result, replay_evidence = replayed['forehead_float32_angle_targets'](*args)
        np.testing.assert_array_equal(result, expected)
        self.assertEqual(replay_evidence, evidence)


if __name__ == '__main__':
    unittest.main()
