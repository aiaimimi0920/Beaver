"""Membership persistence tests: no mesh generation or fitting."""
import copy
import unittest
import numpy as np
from forehead_lower_guard import (forehead_build_lower_guard, forehead_update_lower_guard,
                                  forehead_lower_guard_retains)

class ForeheadLowerGuardTests(unittest.TestCase):
    def setUp(self):
        v=[[i*.01,0.,1.6]for i in range(8)]
        for i,z in ((1,1.52),(2,1.54),(5,1.51),(6,1.55)):v[i][2]=z
        self.p={'faces':[list(range(8))], 'vertices':v, 'mouth_height':1.5,
                'paths':{'blue':[0,1,2,3], 'purple':[4,5,6,7]}}
        self.guard=forehead_build_lower_guard(self.p,v)
        self.p['lower_turn_guard']=self.guard
    def test_upward_crossing_does_not_remove_and_downward_adds(self):
        q=copy.deepcopy(self.p);q['vertices'][1][2]=1.55;q['vertices'][2][2]=1.51
        guard=forehead_update_lower_guard(q,np.asarray(q['vertices']),self.guard)
        self.assertEqual(len(self.guard['triples']),2);self.assertEqual(len(guard['triples']),3)
        self.assertIn([0,1,2],guard['triples']);self.assertIn([1,2,3],guard['triples'])
        q['lower_turn_guard']=guard;forehead_lower_guard_retains(self.p,q)
    def test_deletion_and_missing_new_entrant_rejected(self):
        q=copy.deepcopy(self.p);q['lower_turn_guard']['triples'].pop()
        with self.assertRaisesRegex(ValueError,'dropped'):forehead_lower_guard_retains(self.p,q)
        q=copy.deepcopy(self.p);q['vertices'][2][2]=1.51
        with self.assertRaisesRegex(ValueError,'newly entering'):forehead_lower_guard_retains(self.p,q)
        del q['lower_turn_guard']
        with self.assertRaisesRegex(ValueError,'removed'):forehead_lower_guard_retains(self.p,q)
    def test_topology_change_requires_explicit_rebuild(self):
        q=copy.deepcopy(self.p);q['faces'][0]=q['faces'][0][1:]+q['faces'][0][:1]
        with self.assertRaisesRegex(ValueError,'explicit'):forehead_update_lower_guard(q,q['vertices'],self.guard)
        rebuilt=forehead_build_lower_guard(q,q['vertices'],self.guard['triples'],reason='Explicit source-owned topology rebuild')
        self.assertEqual(rebuilt['triples'],self.guard['triples'])
        with self.assertRaisesRegex(ValueError,'rebasing'):forehead_build_lower_guard(q,q['vertices'],[[0,4,7]])
    def test_cutoff_change_cannot_reset_membership(self):
        q=copy.deepcopy(self.p);q['mouth_height']+=.1
        with self.assertRaisesRegex(ValueError,'cutoff'):forehead_update_lower_guard(q,q['vertices'],self.guard)

if __name__=='__main__':unittest.main()
