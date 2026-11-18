"""Corpus-free detector tests. They do not assert topology/geometry feasibility."""
import copy
import unittest
import numpy as np
from forehead_lower_rail_capacity import forehead_lower_rail_capacity, forehead_lower_rail_candidate_demands, forehead_lower_rail_lateral_transits

class ForeheadLowerRailCapacityTests(unittest.TestCase):

    def test_five_identity_permutations_and_immutable_baseline(self):
        v = np.column_stack([np.array([0, 1, 2, 3, 4, 9]) * 0.01, np.zeros((6, 2))])
        uv = v[:, [0, 2]]
        sample = lambda x: np.column_stack([x[:, 0] * 0.1, np.zeros((len(x), 2))])
        current = sample(uv)
        frozen = current.copy()
        frozen[-2, 0] -= 0.00049
        before = frozen.copy()
        paths = {'rail': list(range(len(v)))}
        base = forehead_lower_rail_capacity(v, paths, 'rail', uv, current, frozen, sample, {0, 5})
        self.assertEqual(base['blocked_vertices'], [4])
        self.assertEqual(base['segment'], (1, 5))
        for seed in range(5):
            perm = np.random.default_rng(seed).permutation(len(v))
            inv = np.argsort(perm)
            out = forehead_lower_rail_capacity(v[inv], {'rail': [int(perm[i]) for i in paths['rail']]},
                'rail', uv[inv], current[inv], frozen[inv], sample, {int(perm[0]), int(perm[5])})
            self.assertEqual(out['blocked_vertices'], [int(perm[4])])
            self.assertEqual(out['segment'], (int(perm[1]), int(perm[5])))
        np.testing.assert_array_equal(frozen, before)
        safe = forehead_lower_rail_capacity(v, paths, 'rail', uv, current, current, sample, {0, 5})
        self.assertIsNone(safe['segment'])
        with self.assertRaises(ValueError):
            forehead_lower_rail_capacity(v, paths, 'rail', uv, current, frozen, lambda q: None, {0, 5})

    def test_recruited_transit_feedback_does_not_ban_all_old_nodes(self):
        v = np.array([[0, 0, 0], [1, 0, 0], [2, 0, 0], [1, 0, 0.1], [1, 0, 5]], float)
        uv = v[:, [0, 2]]
        fields = np.zeros((5, 3))
        sample = lambda x: np.zeros((len(x), 3))
        original = [0, 1, 2]
        candidate = [0, 3, 4, 9, 2]
        demand = forehead_lower_rail_candidate_demands(v, original, candidate, uv, fields, sample, 5)
        self.assertEqual(demand['suggested_search_exclusions'], [])
        self.assertEqual([r['vertex'] for r in demand['newly_recruited_existing_transits']], [3, 4])
        lateral = forehead_lower_rail_lateral_transits(v, original, candidate, 5)
        self.assertEqual(lateral['suggested_search_exclusions'], [4])
        self.assertFalse(lateral['rows'][0]['outside_local_guess'])
if __name__ == '__main__':
    unittest.main()
