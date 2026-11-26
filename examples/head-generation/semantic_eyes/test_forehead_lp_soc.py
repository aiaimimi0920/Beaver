import unittest
import numpy as np
from scipy import sparse
from forehead_lp_restore import forehead_restore_linearized as solve
class Tests(unittest.TestCase):
 def case(self,**kw):
  settings=dict(start=np.array([0.,0.]),radii=np.array([1.,1.]),constraints=lambda x:np.array([x[0]-.5,.001+x[1]-x[0]**2]),jacobian=lambda x:sparse.csr_matrix([[1.,0.],[-2*x[0],1.]]),admit=lambda x:bool(x[0]>=.5 and .001+x[1]-x[0]**2>0),orientation_rows=(1,2),domain=lambda x:True,max_iterations=1,soc_source_domain=lambda x:True,soc_orientation_floor=np.array([.0005]));settings.update(kw);return solve(**settings)
 def test_SOC_exact_domain(self):
  r,m=self.case();self.assertGreaterEqual(.001+r.x[1]-r.x[0]**2,.0005);self.assertTrue(np.all(abs(r.x)<=1));self.assertFalse(r.success)
  records=[t for row in m['steps']for t in row['trials']if 'SOC'in t];self.assertEqual(len(records),1);self.assertLessEqual(len(records[0]['SOC']),2);self.assertEqual(records[0]['alpha'],.5)
 def test_source_invalid_skips(self):
  r,m=self.case(soc_source_domain=lambda x:False);self.assertFalse(any('SOC'in t for row in m['steps']for t in row['trials']))
 def test_bad_floor(self):
  for f in ([0.],[-1.],[np.nan],[.002]):
   with self.assertRaises(ValueError):self.case(soc_orientation_floor=np.array(f))
 def test_no_false_success(self):
  r,m=self.case(admit=lambda x:False);self.assertFalse(r.success)
 def test_disabled_legacy(self):
  r,m=self.case(soc_source_domain=None,soc_orientation_floor=None);self.assertFalse(any('SOC'in t for row in m['steps']for t in row['trials']))
if __name__=='__main__':unittest.main()
