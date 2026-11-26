import copy
import math
import unittest
import numpy as np
from forehead_complete_guide_stage import ForeheadCompleteGuideFairingStage, forehead_prepare_complete_guide_stage


def fixture(count=6, width=.04, depth=.012):
    t = np.linspace(.2, 1.3, count)
    points = np.array([[width*s, depth*math.sin(s), row*.03+.012*math.cos(s)+.002*s]
                       for row in range(5) for s in t], np.float32).astype(float)
    paths = {r: list(range(j*count, (j+1)*count)) for j, r in enumerate(['red', 'orange', 'green', 'blue', 'purple'])}
    faces = [[j*count+i, j*count+i+1, (j+1)*count+i+1, (j+1)*count+i] for j in range(4) for i in range(count-1)]
    proposal = {'vertices': points, 'paths': paths, 'faces': faces, 'support': [],
                'protected': sorted({i for p in paths.values() for i in (p[0], p[-1])})}
    config = {'version': 1, 'max_evaluations': 1000, 'reduction': .001,
              'frames': {'front': [[1, 0, 0], [0, 0, 1]],
                         'right45': [[2**-.5, 2**-.5, 0], [0, 0, 1]],
                         'right90': [[0, 1, 0], [0, 0, 1]]}}
    return points, proposal, config


class CompleteGuideTests(unittest.TestCase):
    def test_invalid_ids_rejected_without_truncation(self):
        p, proposal, config = fixture()
        for location in ('path', 'face', 'support', 'protected'):
            for bad in (True, np.bool_(False), 1.5, 1., float('nan'), -1, len(p)):
                with self.subTest(location=location, bad=bad):
                    broken = copy.deepcopy(proposal)
                    if location == 'path':
                        broken['paths']['red'][1] = bad
                    elif location == 'face':
                        broken['faces'][0][1] = bad
                    elif location == 'support':
                        broken['support'] = [bad]
                    else:
                        broken['protected'][0] = bad
                    with self.assertRaisesRegex(ValueError, 'integer IDs'):
                        ForeheadCompleteGuideFairingStage(broken, p, 'source', config)

    def test_capture_validates_and_tightens_saved_geometry_only(self):
        p, proposal, config = fixture()
        original = ForeheadCompleteGuideFairingStage(proposal, p, 'source', config)
        continuous = p.copy()
        continuous[2, 1] += 1e-11
        saved = original.measure(continuous.astype(np.float32).astype(float))
        working = original.measure(continuous)
        stricter = np.flatnonzero(working < saved)
        self.assertTrue(len(stricter))
        caps = np.maximum(saved, working)
        caps[stricter[0]] = working[stricter[0]]
        proposal['complete_guide_fairing_guard'] = {'version': 1, 'identity': original.identity,
            'source_identity': 'source', 'config': config, 'reference': original.reference,
            'caps': caps.tolist(), 'energy_cap': max(original.energy_from(saved), original.energy_from(working))}
        retained = ForeheadCompleteGuideFairingStage(proposal, p, 'source', None)
        self.assertTrue(retained.accepts(continuous))
        with self.assertRaisesRegex(ValueError, 'unadmitted'):
            retained.capture(proposal, continuous)
        proposal['complete_guide_fairing_guard']['caps'] = saved.tolist()
        retained = ForeheadCompleteGuideFairingStage(proposal, p, 'source', None)
        captured = retained.capture(proposal, continuous)
        np.testing.assert_array_equal(captured['caps'], saved)
        proposal['complete_guide_fairing_guard'] = captured
        later = ForeheadCompleteGuideFairingStage(proposal, p, 'source', None)
        self.assertTrue(later.accepts(p))

    def test_disabled_preserves_old_pipeline(self):
        p, proposal, _ = fixture()
        self.assertIsNone(forehead_prepare_complete_guide_stage(proposal, p, 'source'))

    def test_different_generated_shapes(self):
        for count, width, depth in [(5, .035, .012), (8, .06, .018)]:
            p, proposal, config = fixture(count, width, depth)
            stage = ForeheadCompleteGuideFairingStage(proposal, p, 'source', config)
            self.assertEqual(stage.count, 62)
            self.assertTrue(np.all(stage.constraints(p)[:-1] == 0))
            self.assertFalse(stage.accepts(p), 'No-op cannot satisfy positive reduction')

    def test_full_endpoint_join_is_measured(self):
        p, proposal, config = fixture()
        stage = ForeheadCompleteGuideFairingStage(proposal, p, 'source', config)
        changed = p.copy()
        changed[proposal['paths']['blue'][1], 1] += .005
        self.assertFalse(np.array_equal(stage.measure(p), stage.measure(changed)))
        self.assertFalse(stage.accepts(changed))

    def test_missing_role_and_nonedge_rejected(self):
        p, proposal, config = fixture()
        broken = copy.deepcopy(proposal)
        del broken['paths']['red']
        with self.assertRaises(ValueError):
            ForeheadCompleteGuideFairingStage(broken, p, 'source', config)
        broken = copy.deepcopy(proposal)
        del broken['paths']['red'][2]
        with self.assertRaises(ValueError):
            ForeheadCompleteGuideFairingStage(broken, p, 'source', config)

    def test_saved_reference_not_continuous_predictor(self):
        p, proposal, config = fixture()
        continuous = p + 1e-10
        stage = ForeheadCompleteGuideFairingStage(proposal, continuous, 'source', config)
        np.testing.assert_array_equal(stage.reference, stage.measure(continuous.astype(np.float32).astype(float)))

    def test_stale_source_and_topology_rejected(self):
        p, proposal, config = fixture()
        stage = ForeheadCompleteGuideFairingStage(proposal, p, 'source', config)
        proposal['complete_guide_fairing_guard'] = {'version': 1, 'identity': stage.identity,
            'source_identity': 'source', 'config': config, 'reference': stage.reference,
            'caps': stage.caps.tolist(), 'energy_cap': stage.energy_from(stage.measure(p))}
        retained = forehead_prepare_complete_guide_stage(proposal, p, 'source')
        self.assertTrue(retained.accepts(p))
        with self.assertRaises(ValueError):
            forehead_prepare_complete_guide_stage(proposal, p, 'other-source')
        proposal['support'] = [2]
        with self.assertRaises(ValueError):
            forehead_prepare_complete_guide_stage(proposal, p, 'source')

    def test_budget_counts_derivative_and_its_samples(self):
        p, proposal, config = fixture()
        stage = ForeheadCompleteGuideFairingStage(proposal, p, 'source', config)
        def evaluate(x):
            stage.charge('source_evaluate_including_cached_and_FD')
            q = p.copy()
            q[2, 0] += x[0]*1e-4
            return (q,)
        stage.parameter_gradient(np.zeros(1), evaluate)
        self.assertEqual(stage.total_calls, 6)
        self.assertEqual(stage.calls['objective'], 2)
        stage.config['max_evaluations'] = stage.total_calls
        with self.assertRaisesRegex(ValueError, 'budget exhausted'):
            stage.objective(p)

    def test_unadmitted_capture_rejected(self):
        p, proposal, config = fixture()
        stage = ForeheadCompleteGuideFairingStage(proposal, p, 'source', config)
        with self.assertRaises(ValueError):
            stage.capture(proposal, p)


if __name__ == '__main__':
    unittest.main()
