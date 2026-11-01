"""Real atlas checks with explicitly mocked expensive construction calls."""
import copy
import unittest
from unittest.mock import patch
import numpy as np
from forehead_initial_ready import initialize_forehead_shared
from forehead_author_atlas import ForeheadFrozenAuthorAtlas
from test_forehead_gap_fixture import make_forehead_gap_fixture


class ForeheadInitialReadyTests(unittest.TestCase):
    def setUp(self):
        self.author, self.proposal, references = make_forehead_gap_fixture()
        self.author['open_delta'] = np.zeros_like(self.author['vertices']).tolist()
        self.author['ordinals'] = {}
        self.atlas = ForeheadFrozenAuthorAtlas(self.author, references)

    def construction(self, proposal):
        return {'admitted': True, 'status': 'local_initializer_admitted',
                'proposal': copy.deepcopy(proposal)}

    def reversed_tail(self, pin):
        p = copy.deepcopy(self.proposal)
        path = p['paths']['orange']
        node = path[-1]+1
        path.insert(-1, node)
        if pin:
            p['protected'].append(node)
        return p, node

    def test_valid_atlas_is_not_final_model_acceptance(self):
        before = copy.deepcopy(self.proposal)
        with patch('forehead_initial_ready.forehead_initial_construct',
                   return_value=self.construction(self.proposal)) as factory:
            result = initialize_forehead_shared(self.author, {}, atlas=self.atlas)
        self.assertTrue(result['admitted'])
        self.assertTrue(result['local_init_only'])
        self.assertEqual(result['status'], 'initial_atlas_ready')
        self.assertEqual(result['proposal']['initial_atlas_preflight']['checks'][-1]
                         ['source_endpoints_checked'], 20500)
        self.assertEqual(self.proposal, before)
        self.assertEqual(factory.call_count, 1)

    def test_only_proved_pinned_reversal_drives_rebuild(self):
        bad, node = self.reversed_tail(True)
        with patch('forehead_initial_ready.forehead_initial_construct', side_effect=[
                self.construction(bad), self.construction(self.proposal)]) as factory:
            result = initialize_forehead_shared(self.author, {}, atlas=self.atlas)
        self.assertTrue(result['admitted'])
        self.assertEqual(factory.call_args_list[1].kwargs['blocked_transits'], (node,))
        self.assertEqual(len(result['history']), 2)
        self.assertIn(node, bad['protected'])

    def test_movable_defect_is_not_masked_or_retried_as_fixed(self):
        bad, node = self.reversed_tail(False)
        with patch('forehead_initial_ready.forehead_initial_construct',
                   return_value=self.construction(bad)) as factory:
            result = initialize_forehead_shared(self.author, {}, atlas=self.atlas)
        self.assertFalse(result['admitted'])
        self.assertEqual(result['status'], 'initial_atlas_conditioning_required')
        self.assertIn(node, result['proposal']['paths']['orange'])
        self.assertEqual(factory.call_count, 1)

    def test_graph_rejection_does_not_return_ready_geometry(self):
        with patch('forehead_initial_ready.forehead_initial_construct', return_value={
                'admitted': False, 'status': 'joint_graph_rejected'}):
            result = initialize_forehead_shared(self.author, {}, atlas=self.atlas)
        self.assertFalse(result['admitted'])
        self.assertNotIn('proposal', result)

    def test_caller_cannot_supply_unproved_transit_exclusions(self):
        with self.assertRaisesRegex(ValueError, 'immutable sector evidence'):
            initialize_forehead_shared(self.author, {}, atlas=self.atlas,
                                        construct_options={'blocked_transits': [1]})


if __name__ == '__main__':
    unittest.main()
