import unittest,copy
from types import SimpleNamespace
import numpy as np
from forehead_arc_quantization import forehead_arc_quantization_targets as target
class Tests(unittest.TestCase):
 def setUp(self):
  self.p=np.array([[0.,0,0],[0,1.0500001,0]])
  self.s=SimpleNamespace(paths={'rail':[0,1]},groups=[(0,3,[0,1])],count=3,reference={'rail':1000.})
 def test_measured_scaled(self):
  before=copy.deepcopy(self.s);x=self.p.copy();t,e=target(x,self.s,2,np.array([1,1,1,1,2.]));self.assertEqual(len(e),1);self.assertEqual(e[0]['row'],4);self.assertAlmostEqual(2*t[4],max(float(np.float32(1.0500001))-1.0500001,0)+1e-7);np.testing.assert_array_equal(x,self.p);self.assertEqual(vars(before),vars(self.s))
 def test_safe_no_requests(self):
  self.p[1,1]=1.;t,e=target(self.p,self.s,0,np.ones(3));self.assertFalse(t.any());self.assertEqual(e,[])
 def test_invalid_scales_safety_offset(self):
  for scales,safety,offset in [(np.zeros(3),1e-7,0),(np.ones(3),0,0),(np.ones(3),float('nan'),0),(np.ones(3),1e-7,.5)]:
   with self.assertRaises(ValueError):target(self.p,self.s,offset,scales,safety)
 def test_bad_groups(self):
  for group in [(0,4,[0,1]),(0,3,[1,0]),(0.,3,[0,1])]:
   self.s.groups=[group]
   with self.assertRaises(ValueError):target(self.p,self.s,0,np.ones(4))
 def test_bad_reference_and_range(self):
  with self.assertRaises(ValueError):target(self.p,self.s,2,np.ones(3))
  self.s.reference['rail']=float('nan')
  with self.assertRaises(ValueError):target(self.p,self.s,0,np.ones(3))
if __name__=='__main__':unittest.main()
