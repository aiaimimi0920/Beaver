"""Aperture and cavity contract tests for the bounded depth-only translation."""
import math
import unittest
from lip_seam_shift import shift_lip_seam
from source_lip_seam_flow import adapt
from test_lip_joint_flow import JointLipTest


class SeamTest(unittest.TestCase):
    def sample(self):
        vertices, rings, angles, _ = JointLipTest().sample_half()
        return vertices, rings, angles

    def apply(self, vertices, rings, angles, guarded, amount=.00075):
        return shift_lip_seam(vertices, rings, angles, guarded, 1.,
                              dict(upper_lip_edge_depth_delta=0.,
                                   lower_lip_edge_depth_delta=0., mouth_half_width=.02), amount)

    def test_only_aperture_depth_changes(self):
        vertices, rings, angles = self.sample()
        before = vertices.copy()
        calls = []
        def guard(x, y, **kw):
            calls.append(kw)
            self.assertNotIn("upper_lip_local", kw)
            return kw["interpolated_depth"]
        result = self.apply(vertices, rings, angles, guard)
        self.assertTrue(result["xy_fixed"])
        for i, (old, new) in enumerate(zip(before, vertices)):
            self.assertEqual((old[0], old[2]), (new[0], new[2]))
            expected = -.00075*(1-min(1.,abs(old[0])/.02)**4)**2 if i in rings[0] else 0.
            self.assertAlmostEqual(new[1]-old[1], expected)
        self.assertEqual(len(calls), 2*len(angles))

    def test_original_reference_failure_is_transactional(self):
        vertices, rings, angles = self.sample()
        before = vertices.copy()
        def reject(*args, **kwargs):
            raise ValueError("original reference")
        with self.assertRaisesRegex(ValueError, "original reference"):
            self.apply(vertices, rings, angles, reject)
        self.assertEqual(vertices, before)

    def test_unauthorized_or_nonfinite_amount_is_rejected(self):
        for amount in (.00150001, -.00150001, math.inf, math.nan):
            vertices, rings, angles = self.sample()
            before = vertices.copy()
            with self.assertRaises(ValueError):
                self.apply(vertices, rings, angles, lambda **kw: 0, amount)
            self.assertEqual(vertices, before)

    def test_source_contract_fails_closed(self):
        with self.assertRaises(ValueError):
            adapt({"shell.py": "def build_face():\n    pass\n"})


if __name__ == "__main__":
    unittest.main()
