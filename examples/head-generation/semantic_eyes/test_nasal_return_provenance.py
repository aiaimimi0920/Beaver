"""Synthetic strips exercise strict generated-return identity, not native evidence."""
import copy
from pathlib import Path
import sys
import unittest
import numpy as np
sys.path.insert(0, str(Path(__file__).parent))
from nasal_return_provenance import verify_nasal_return_provenance


class ReturnProvenanceTests(unittest.TestCase):
    def setUp(self):
        self.v = np.asarray([[x+s, 0, z] for s in (0, 10) for x in (0, 1) for z in (0, 1, 2)], float)
        self.f = [(0,1,4,3),(1,2,5,4),(6,7,10,9),(7,8,11,10)]
        self.r = [1]*4
        self.record = {'open_back': True, 'generated_strip_provenance': {
            'schema_version': 1,
            'coordinate_frame': 'input Mesh local; unchanged coordinates required at consumption',
            'parent_rim_pairs': [{'parent':self.v[a].tolist(),'rim':self.v[b].tolist()} for a,b in [(1,4),(7,10)]],
            'support_cycles': [{'positions':self.v[list(f)].tolist(),'region':1} for f in self.f],
            'boundary_edges': [self.v[list(e)].tolist() for e in [(3,4),(4,5),(9,10),(10,11)]]}}
    def verify(self, v=None, f=None, r=None, record=None):
        return verify_nasal_return_provenance(self.v if v is None else v, self.f if f is None else f,
            self.r if r is None else r, self.record if record is None else record, [(1,4),(7,10)])
    def test_exact_surviving_strip_and_cyclic_order(self):
        self.assertEqual(len(self.verify()['pairs']), 2)
        self.verify(f=[f[1:]+f[:1] for f in self.f])
    def test_moved_duplicate_reversed_or_changed_owner_rejected(self):
        moved=self.v.copy();moved[4,0]+=.000001
        duplicate=np.vstack((self.v,self.v[4]))
        for v,f,r in [(moved,self.f,self.r),(duplicate,self.f,self.r),
                      (self.v,[self.f[0][::-1]]+self.f[1:],self.r),
                      (self.v,self.f,[2,1,1,1])]:
            with self.assertRaises(ValueError): self.verify(v,f,r)
    def test_missing_generator_proof_or_wrong_boundary_rejected(self):
        for key in ('parent_rim_pairs','support_cycles','boundary_edges'):
            record=copy.deepcopy(self.record);record['generated_strip_provenance'][key]=[]
            with self.assertRaises(ValueError): self.verify(record=record)
        record=copy.deepcopy(self.record);record['open_back']=False
        with self.assertRaises(ValueError): self.verify(record=record)

if __name__ == '__main__': unittest.main()
