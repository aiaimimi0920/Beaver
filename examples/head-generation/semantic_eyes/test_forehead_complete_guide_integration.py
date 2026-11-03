import copy
import unittest
import numpy as np
from test_forehead_complete_guide_stage import fixture
from forehead_complete_guide_stage import ForeheadCompleteGuideFairingStage
from forehead_complete_guide_fit import (forehead_install_complete_stage, forehead_complete_objective,
    forehead_complete_fit_capture, forehead_complete_fit_publish)
from forehead_fairing_guard import forehead_fairing_guard_retains
from forehead_fairing_continuation import continue_forehead_fairing
from forehead_shared_consumer import forehead_consumer_options
from forehead_shared_fit import fit_forehead_shared_constraints
from source_nasal_spacing import adapt
from feature_flow_recipe import retained_sources


class CompleteGuideIntegrationTests(unittest.TestCase):
    def test_publish_evidence_includes_calls_after_guard_capture(self):
        p, proposal, config = fixture()
        original = ForeheadCompleteGuideFairingStage(proposal, p, 'source', config)
        proposal['complete_guide_fairing_guard'] = {'version': 1, 'identity': original.identity,
            'source_identity': 'source', 'config': config, 'reference': original.reference,
            'caps': original.caps.tolist(), 'energy_cap': original.energy_from(original.measure(p))}
        stage = ForeheadCompleteGuideFairingStage(proposal, p, 'source', None)
        result = forehead_complete_fit_capture(stage, proposal, p)
        captured_count = stage.total_calls
        self.assertNotIn('metrics', result)
        stage.charge('source_evaluate_including_cached_and_FD')
        stage.objective(p)
        proposal['metrics'] = {}
        forehead_complete_fit_publish(proposal, result, stage)
        evidence = proposal['metrics']['complete_guide_fairing']
        self.assertEqual(evidence['counted_evaluations'], captured_count+2)
        self.assertEqual(evidence['counted_evaluations'], sum(evidence['evaluation_kinds'].values()))
        for _ in range(stage.config['max_evaluations']-stage.total_calls):
            stage.charge('late_source_evaluation')
        with self.assertRaisesRegex(ValueError, 'budget'):
            stage.objective(p)

    def test_actual_fit_rejects_bypassed_existing_stages(self):
        _, _, config = fixture()
        with self.assertRaisesRegex(ValueError, 'physical/fairing gates'):
            fit_forehead_shared_constraints([], [], {}, complete_guide_fairing=config)

    def test_installer_adds_rows_and_binds_immutable_source(self):
        p, proposal, config = fixture()
        proposal['topology'] = {'faces': copy.deepcopy(proposal['faces'])}
        layout = {'count': 17}
        stage = forehead_install_complete_stage(proposal, p, (p, np.zeros_like(p)), layout, config)
        self.assertEqual(layout['complete_guide_fairing'], (17, 79))
        self.assertEqual(layout['count'], 79)
        self.assertEqual(len(stage.source_identity), 64)

    def test_original_objective_and_derivative_are_retained(self):
        p, proposal, config = fixture()
        stage = ForeheadCompleteGuideFairingStage(proposal, p, 'source', config)
        calls = []
        def old(x):
            calls.append('old_objective')
            return 7.
        def gradient(x):
            calls.append('old_gradient')
            return np.zeros_like(x)
        def evaluate(x):
            stage.charge('source_evaluate_including_cached_and_FD')
            q = p.copy()
            q[2, 0] += x[0]*1e-4
            return (q,)
        fun, jac = forehead_complete_objective(stage, config, old, gradient, evaluate, np.zeros(1))
        self.assertAlmostEqual(fun(np.zeros(1)), 7.+stage.energy_from(stage.measure(p)))
        self.assertTrue(np.isfinite(jac(np.zeros(1))).all())
        self.assertEqual(calls, ['old_objective', 'old_gradient'])
        self.assertEqual(forehead_complete_objective(None, None, old, gradient, evaluate, np.zeros(1)), (old, gradient))

    def test_later_stage_cannot_drop_complete_guard(self):
        p, proposal, config = fixture()
        stage = ForeheadCompleteGuideFairingStage(proposal, p, 'source', config)
        proposal['complete_guide_fairing_guard'] = {'version': 1, 'identity': stage.identity,
            'source_identity': 'source', 'config': config, 'reference': stage.reference,
            'caps': stage.caps.tolist(), 'energy_cap': stage.energy_from(stage.measure(p))}
        proposal['complete_guide_fairing_reference'] = copy.deepcopy(stage.reference)
        forehead_fairing_guard_retains(proposal, copy.deepcopy(proposal))
        broken = copy.deepcopy(proposal)
        del broken['complete_guide_fairing_guard']
        with self.assertRaises(ValueError):
            forehead_fairing_guard_retains(proposal, broken)

    def test_active_consumer_and_continuation_forward_explicit_config(self):
        _, _, config = fixture()
        options = forehead_consumer_options({'mode': 'fresh', 'fairing_stages': 1, 'complete_guide_fairing': config})
        self.assertEqual(options['complete_guide_fairing'], config)
        with self.assertRaises(ValueError):
            forehead_consumer_options({'mode': 'fresh', 'complete_guide_fairing': config})
        received = []
        def fitter(v, d, p, **kwargs):
            received.append(kwargs['complete_guide_fairing'])
            p['metrics'] = {'guide_fairing': {'requested_reduction': .001}}
            return p
        continue_forehead_fairing([], [], {'vertices': [[0, 0, 0]], 'metrics': {}}, {},
                                 fitter=fitter, complete_guide_fairing=config)
        self.assertEqual(received, [config])

    def test_dense_complete_rows_cover_incident_support_variables(self):
        from types import SimpleNamespace
        from forehead_fit_sparsity import forehead_fit_constraint_sparsity
        layout = {'count': 4, 'field': (0, 2), 'complete_guide_fairing': (2, 4)}
        stage = SimpleNamespace(nodes=[8, 21])
        empty = SimpleNamespace(groups=[])
        sparse = forehead_fit_constraint_sparsity(layout, 4, [8, 21], [], [],
            None, None, stage, empty, SimpleNamespace(specs=[]), empty).toarray()
        np.testing.assert_array_equal(sparse[2:4], np.ones((2, 4)))
        np.testing.assert_array_equal(sparse[4:], np.eye(4))

    def test_generated_flattened_package_executes_new_stage(self):
        pipeline = '''def pipeline():
        report.update(refine_nasal_source(data))
        from shell import refine_chin_source
        report.update(refine_chin_source(data,report["chin_source_anchors"]))
'''
        out = adapt({'pipeline.py': pipeline, 'shell.py': ''})
        namespace = {}
        exec(compile(out['shell.py'], 'generated-complete-guide-shell', 'exec'), namespace)
        p, proposal, config = fixture()
        stage = namespace['ForeheadCompleteGuideFairingStage'](proposal, p, 'source', config)
        self.assertEqual(stage.count, 62)
        self.assertFalse(stage.accepts(p))
        sources = retained_sources()
        self.assertIn('forehead_complete_guide_stage.py', sources)
        self.assertIn('forehead_complete_guide_fit.py', sources)


if __name__ == '__main__':
    unittest.main()
