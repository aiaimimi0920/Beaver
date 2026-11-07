"""Controller-only test with explicitly mocked expensive stages."""
import copy, unittest
from unittest.mock import patch
import numpy as np
import forehead_lower_rail_recovery as m

class ForeheadFeedbackPolicyTests(unittest.TestCase):

    def test_diagnostic_demand_is_not_a_relaxed_true_field_guard(self):
        v = [[0, 0, 0], [1, 0, 0], [1, 0, 1], [0, 0, 1]]
        p = {'vertices': v, 'faces': [[0, 1, 2, 3]], 'source_faces': [[0, 1, 2, 3]], 'source_chart': v,
            'open_delta': [[0, 0, 0]] * 4, 'lineage': {}, 'paths': {'rail': [0, 1, 2]}, 'protected': [0, 2],
            'topology': {'lineage': {}, 'face_owners': [0]}}
        before = copy.deepcopy(p)
        with patch.object(m, 'forehead_lower_rail_capacity', return_value={'blocked_vertices': [1],
            'segment': (0, 2)}), patch.object(m, 'forehead_pattern_select_cavity',
            return_value=[0]), patch.object(m, 'forehead_pattern_catalogue', return_value={}), patch.object(m,
            'forehead_pattern_solve', return_value={'feasible': True, 'status': 'TEST ONLY'}), patch.object(m,
            'forehead_pattern_materialize', return_value=(copy.deepcopy(p), {})), patch.object(m,
            'forehead_lower_rail_candidate_demands',
            return_value={'suggested_search_exclusions': [3]}), patch.object(m,
            'forehead_lower_rail_birth_geometry', return_value=({'admitted': False,
            'reason': ['true field guard rejected']}, {})) as birth:
            args = (p, np.array(v, float), np.zeros((4, 3)), {'role': [1] * 4}, {}, set(), [0], 'rail')
            rejected = m.forehead_lower_rail_recover(*args, max_graph_attempts=1,
                target_feedback_policy='exclude')
            self.assertEqual(rejected['status'], 'recruited_transit_capacity_budget_exhausted')
            self.assertFalse(birth.called)
            tested = m.forehead_lower_rail_recover(*args, max_graph_attempts=1,
                target_feedback_policy='diagnose')
            self.assertEqual(tested['status'], 'local_birth_geometry_rejected')
            self.assertFalse(tested['admitted'])
            self.assertTrue(birth.called)
        self.assertEqual(p, before)
if __name__ == '__main__':
    unittest.main()
