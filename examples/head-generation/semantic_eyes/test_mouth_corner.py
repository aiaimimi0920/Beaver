"""Small pure contracts; full real-source audits remain separate evidence."""
from pathlib import Path
import sys
import unittest
import numpy as np
sys.path.insert(0, str(Path(__file__).parent))
from mouth_corner_rule import propose_lower_mouth_corner
from mouth_corner_native import prepare_mouth_corner
from mouth_corner_shapes import guard_mouth_corner_shapes


class MouthCornerTests(unittest.TestCase):
    def test_missing_chin_commit_rejected(self):
        for chin in ({}, {'transaction': {'committed': True}}):
            with self.assertRaisesRegex(ValueError, 'completed current chin'):
                prepare_mouth_corner(np.zeros((4, 3)), [], [], {}, [], chin, lambda p: 0.)

    def test_invalid_budget_and_missing_aperture(self):
        for budget in (0, -1, 2.1, float('nan')):
            with self.assertRaisesRegex(ValueError, 'budget'):
                propose_lower_mouth_corner(np.zeros((4, 3)), [], [], [], [], interface_budget_deg=budget)
        with self.assertRaisesRegex(ValueError, 'oral aperture'):
            propose_lower_mouth_corner(np.zeros((4, 3)), [], [], [], [])

    def test_duplicate_chin_cycles_rejected(self):
        v = np.array([[-1,0,0],[1,0,0],[1,0,1],[-1,0,1]], dtype=float)
        chin = {'transaction': {'committed': True, 'transfer_payload': {'equal': True}},
                'proposal': {'replacement_faces': [[0,1,2,3]] * 37}}
        with self.assertRaisesRegex(ValueError, 'certificate is incomplete'):
            prepare_mouth_corner(v, [(0,1,2,3)], [1], {'protected': set()}, [], chin, lambda p: 0.)

    def test_mouth_targets_recomputed_with_open_selector(self):
        v = np.array([[0,0,0],[1,0,0],[1,0,1],[0,0,1]], dtype=np.float32)
        opening = np.zeros_like(v); opening[:, 1] = .001
        seen, targets = [], []
        def selector(points, face, pose):
            seen.append(pose.copy())
            return {0: 1.}
        def motion(point, target):
            targets.append(target)
            return (0., 0., 0.)
        proof = guard_mouth_corner_shapes(v, [(0,1,2,3)], v, {0}, opening, opening, selector, motion)
        self.assertEqual(proof['orientation_states'], 81)
        self.assertEqual(set(targets), {'MouthRound','MouthSpread','MouthPucker'})
        self.assertTrue(all(np.array_equal(p, v + opening) for p in seen))

    def test_nonfinite_generated_target_rejected(self):
        v = np.zeros((4, 3), dtype=np.float32)
        with self.assertRaisesRegex(ValueError, 'generated mouth target'):
            guard_mouth_corner_shapes(v, [(0,1,2,3)], v, {0}, v, v,
                lambda *args: {0: 1.}, lambda *args: (float('nan'),0.,0.))


if __name__ == '__main__':
    unittest.main()
