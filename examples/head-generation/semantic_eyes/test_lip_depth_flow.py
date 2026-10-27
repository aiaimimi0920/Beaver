"""Depth-only refinement keeps sampling and uses transactional strict guards."""
import unittest
import math
from lip_depth_solver import refine_lip_depth
from source_lip_depth_flow import adapt
import test_lip_joint_flow as joint_cases


class LipDepthTest(unittest.TestCase):
    def fixture(self):
        vertices, rings, angles, _ = joint_cases.JointLipTest().sample_half()
        for r, amount in ((1, .001), (2, -.0005)):
            for k, angle in enumerate(angles):
                p = list(vertices[rings[r][k]])
                p[1] -= amount*max(0., math.sin(angle))**2
                vertices[rings[r][k]] = tuple(p)
        return vertices, rings, angles

    def run_solver(self, vertices, rings, angles, guarded):
        return refine_lip_depth(vertices, rings, angles, lambda x, y: (.08, .08),
                                guarded, 1., {'upper_lip_edge_depth_delta': 0.})

    def test_sampling_and_protected_points_stay_fixed(self):
        vertices, rings, angles = self.fixture()
        before = vertices.copy()
        def guarded(x, y, additional_move=0., upper_lip_local=False):
            if abs(additional_move) > .0015:
                raise ValueError('move')
            return .08+additional_move
        report = self.run_solver(vertices, rings, angles, guarded)
        changed = {i for i, (a, b) in enumerate(zip(before, vertices)) if a != b}
        allowed = {rings[r][k] for r in (1, 2) for k, a in enumerate(angles)
                   if 1e-7 < math.sin(a) < .99999}
        # A fully constrained synthetic ring may stay stationary.
        self.assertTrue(report['xy_fixed'])
        self.assertGreaterEqual(report['iterations'], 1)
        self.assertTrue(changed <= allowed)
        self.assertTrue(all(a[0] == b[0] and a[2] == b[2] for a, b in zip(before, vertices)))
        self.assertLessEqual(max(report['radial_after_deg']), max(report['radial_before_deg'])+.001)

    def test_guard_rejection_is_transactional(self):
        vertices, rings, angles = self.fixture()
        before = vertices.copy()
        def reject(*args, **kwargs):
            raise ValueError('strict final guard')
        with self.assertRaisesRegex(ValueError, 'strict final guard'):
            self.run_solver(vertices, rings, angles, reject)
        self.assertEqual(vertices, before)

    def test_adapter_rejects_missing_hook(self):
        with self.assertRaises(ValueError):
            adapt({'shell.py': 'def build_face():\n    return None\n'})

    def test_native_storage_baseline_uses_same_precision(self):
        import json
        import numpy as np
        from pathlib import Path
        fixture = json.loads(Path(__file__).with_name('lip_depth_storage_fixture.json').read_text())
        positions = Path(__file__).with_name(fixture['positions_file'])
        base = np.array(json.loads(positions.read_text()))
        active = fixture['active']
        records = fixture['provenance']
        angles = fixture['a']
        points = np.array([base[r, k] for r, k in active])
        r, k = active[0]
        ray = base[-1, k, [0, 2]] - base[0, k, [0, 2]]
        t = np.dot(base[r, k, [0, 2]]-base[0, k, [0, 2]], ray)/np.dot(ray, ray)
        rim_scale = records[0][2]/(math.sin(angles[k])**2*(1-t)**2)
        def terms(x, y):
            distance = np.linalg.norm(points[:, [0, 2]]-[x, y], axis=1)
            self.assertLess(float(min(distance)), 1e-6)
            return tuple(records[int(np.argmin(distance))][:2])
        def guarded(x, y, additional_move=0., upper_lip_local=False):
            raw, original = terms(x, y)
            self.assertLessEqual(abs(raw-original), .0042)
            self.assertLessEqual(abs(additional_move), .0015)
            self.assertLessEqual(abs(raw+additional_move-original), .0052 if upper_lip_local else .0042)
            return raw+additional_move
        vertices = base.reshape((-1, 3)).tolist()
        rings = np.arange(len(vertices)).reshape((5, -1)).tolist()
        report = refine_lip_depth(vertices, rings, angles, terms, guarded, 1.,
                                  {'upper_lip_edge_depth_delta': rim_scale})
        self.assertLess(report['max_move_m'], .001)
        self.assertLess(max(report['radial_after_deg']), max(report['radial_before_deg']))
