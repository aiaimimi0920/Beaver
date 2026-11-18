"""Behavioral checks for the joint semantic support redistribution."""
import math
import unittest
from lip_patch_solver import refine_lip_patch
from source_lip_patch_flow import adapt
import test_lip_joint_flow


class LipPatchTests(unittest.TestCase):
    def sample(self):
        vertices, rings, angles, _ = test_lip_joint_flow.JointLipTest().sample_half()
        for row in (1, 2):
            for k, angle in enumerate(angles):
                x, z, y = vertices[rings[row][k]]
                vertices[rings[row][k]] = (x, z-(.001 if row == 1 else .0003)*max(0, math.sin(angle))**2, y)
        return vertices, rings, angles

    def solve(self, vertices, rings, angles, guard):
        return refine_lip_patch(vertices, rings, angles, lambda x, y:(.08, .08), guard,
                                1., {'upper_lip_edge_depth_delta':0.})

    def test_real_solve_changes_only_upper_internal_supports(self):
        vertices, rings, angles = self.sample()
        before = vertices.copy()
        calls = []
        def guard(x, y, additional_move=0., **kw):
            calls.append((additional_move, kw))
            return .08+additional_move
        result = self.solve(vertices, rings, angles, guard)
        allowed = {rings[r][k] for r in (1, 2) for k, a in enumerate(angles) if math.sin(a)>1e-7}
        changed = {i for i, (a, b) in enumerate(zip(before, vertices)) if a != b}
        self.assertTrue(changed)
        self.assertTrue(changed <= allowed)
        for r in (1, 2):
            self.assertEqual(vertices[rings[r][-1]][0], before[rings[r][-1]][0])
        self.assertLess(max(result['radial_after_deg']), max(result['radial_before_deg']))
        self.assertFalse(result['visual_acceptance'])
        self.assertTrue(any(kw.get('upper_lip_local') for _, kw in calls))
        self.assertTrue(any(not kw for _, kw in calls))

    def test_original_guard_failure_is_transactional(self):
        vertices, rings, angles = self.sample()
        before = vertices.copy()
        def reject(*args, **kwargs):
            raise ValueError('Original reference denied')
        with self.assertRaisesRegex(ValueError, 'Original reference denied'):
            self.solve(vertices, rings, angles, reject)
        self.assertEqual(vertices, before)

    def test_nonfinite_input_rejected_without_mutation(self):
        vertices, rings, angles = self.sample()
        vertices[0] = (math.inf, 0., 0.)
        before = vertices.copy()
        with self.assertRaisesRegex(ValueError, 'Non-finite'):
            self.solve(vertices, rings, angles, lambda *a, **kw:0.)
        self.assertEqual(vertices, before)

    def test_source_hook_fails_closed(self):
        with self.assertRaisesRegex(ValueError, 'source contract'):
            adapt({'shell.py':'def build_face():\n    pass\n'})


if __name__ == '__main__':
    unittest.main()
