import unittest
import numpy as np
from forehead_field_jacobian import forehead_field_parameter_jacobian as f
class Tests(unittest.TestCase):
 def test_FD_units(self):
  rng=np.random.default_rng(8);d=rng.normal(size=(4,3))*.001;b=rng.normal(size=(4,3))*.001;D=rng.normal(size=(4,3,2))*.05
  J,flags=f(d,D,b,np.zeros(4,bool));J=J.toarray();self.assertFalse(flags.any())
  for col in range(8):
   i,k=divmod(col,2);h=1e-4;plus=d.copy();minus=d.copy();plus[i]+=D[i,:,k]*h/1000;minus[i]-=D[i,:,k]*h/1000
   fd=-1000*(np.linalg.norm(plus-b,axis=1)-np.linalg.norm(minus-b,axis=1))/(2*h);np.testing.assert_allclose(J[:,col],fd,rtol=1e-7,atol=1e-10)
 def test_zero(self):
  J,flags=f(np.zeros((1,3)),np.ones((1,3,2)),np.zeros((1,3)),np.array([True]));self.assertFalse(flags.any());self.assertTrue((J.toarray()==0).all())
 def test_boundary(self):
  _,flags=f(np.ones((1,3)),np.ones((1,3,2)),np.zeros((1,3)),np.array([True]));self.assertTrue(flags[0])
 def test_bad(self):
  with self.assertRaises(ValueError):f(np.zeros((1,3)),np.ones((1,3,2)),np.zeros((1,3)),[1])
if __name__=='__main__':unittest.main()
