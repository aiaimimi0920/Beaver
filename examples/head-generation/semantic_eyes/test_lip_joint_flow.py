"""Protect strict depth guards and transactional joint-support integration."""
import math
import unittest
from source_lip_joint_flow import adapt
from lip_joint_solver import refine_lip_support


class JointLipTest(unittest.TestCase):
    def test_depth_terms_do_not_apply_fairing_twice(self):
        face = '''def depth(x, y, additional_move=0., upper_lip_local=False, interpolated_depth=None):
    raw = x + y
    result = raw + .001
    result += additional_move
    if interpolated_depth is not None:
        result = interpolated_depth
    original = x + y
    from math import isfinite
    if not all(isfinite(v) for v in (result, original, additional_move)):
        raise ValueError("finite")
    if additional_move and not upper_lip_local:
        raise ValueError("scope")
    if abs(additional_move) > .0015:
        raise ValueError("local")
    if abs(result-original) > (.0052 if upper_lip_local else .0042):
        raise ValueError("total")
    return result
'''
        shell = 'def build():\n    def patch(boundary, kind, closed):\n        rings.append(boundary)\n    return {\n        \"semantic_rows\": levels,\n    }\n'
        updated = adapt({'face.py': face, 'shell.py': shell})
        ns = {}
        exec(updated['face.py'], ns)
        self.assertEqual(ns['lip_depth_terms'](.1, .2), (.30100000000000005, .30000000000000004))
        self.assertAlmostEqual(ns['depth'](.1, .2, .001, True), .302)
        self.assertAlmostEqual(ns['depth'](.1, .2, interpolated_depth=.303), .303)
        for args in ((.1, .2, .001), (.1, .2, .002, True), (math.nan, .2)):
            with self.assertRaises(ValueError):
                ns['depth'](*args)
        with self.assertRaises(ValueError):
            ns['depth'](.1, .2, interpolated_depth=.31)
        self.assertIn('def refine_lip_support(', updated['face.py'])
        self.assertIn('if kind == "mouth":', updated['shell.py'])

    def test_invalid_contract_does_not_mutate_vertices(self):
        vertices = [(0., 0., 0.)]
        original = vertices.copy()
        with self.assertRaises(ValueError):
            refine_lip_support(vertices, [], [], None, None, None, 1., {}, None)
        self.assertEqual(vertices, original)

    def test_nonfinite_input_does_not_mutate_vertices(self):
        vertices = [(0., 0., math.inf)]
        with self.assertRaises(ValueError):
            refine_lip_support(vertices, [[0]] * 5, [0.], None, None, None, 1., {}, None)
        self.assertEqual(vertices, [(0., 0., math.inf)])

    def sample_half(self):
        import numpy as np
        from source_lip_relief_flow import lip_source_ring_fraction
        angles = np.linspace(-np.pi / 2, np.pi / 2, 9)
        inner = np.array([[.02*np.cos(a), -.08, 1.58+.002*np.sin(a)] for a in angles])
        outer = np.array([[.04*np.cos(a), -.08, 1.58+.02*np.sin(a)] for a in angles])
        vertices, rings = [], []
        for base in (0, .5, .75, .875, 1):
            ring = []
            for k, angle in enumerate(angles):
                t = lip_source_ring_fraction(base, angle)
                ring.append(len(vertices))
                vertices.append(tuple(inner[k]*(1-t)+outer[k]*t))
            rings.append(ring)
        return vertices, rings, angles, lip_source_ring_fraction

    def solve_sample(self, vertices, rings, angles, fractions, guarded):
        return refine_lip_support(vertices, rings, angles, fractions,
                                  lambda x, y: (.08, .08), guarded, 1.,
                                  dict(upper_lip_edge_depth_delta=0., lower_lip_edge_depth_delta=0.),
                                  lambda x: 1.58515)

    def test_valid_solve_preserves_all_other_rings_and_midline(self):
        vertices, rings, angles, fractions = self.sample_half()
        before = vertices.copy()
        def guarded(x, y, additional_move=0., upper_lip_local=False, interpolated_depth=None):
            return .08+additional_move if interpolated_depth is None else interpolated_depth
        report = self.solve_sample(vertices, rings, angles, fractions, guarded)
        changed = {i for i, (a, b) in enumerate(zip(before, vertices)) if a != b}
        self.assertTrue(changed)
        self.assertTrue(changed <= set(rings[1][1:-1]))
        self.assertLessEqual(report['corner_turn_after_deg'], report['corner_turn_before_deg']+1)
        self.assertFalse(report['visual_acceptance'])

    def test_strict_guard_failure_leaves_every_vertex_unchanged(self):
        vertices, rings, angles, fractions = self.sample_half()
        before = vertices.copy()
        def reject(*args, **kwargs):
            raise ValueError('Final source guard rejected candidate')
        with self.assertRaisesRegex(ValueError, 'Final source guard'):
            self.solve_sample(vertices, rings, angles, fractions, reject)
        self.assertEqual(vertices, before)
