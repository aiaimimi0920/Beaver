import unittest
import numpy as np
from scipy import sparse
from forehead_lp_restore import forehead_restore_linearized as solve
class Tests(unittest.TestCase):
 def test_hold_feasible(self):
  r,m=solve(np.zeros(2),np.ones(2),lambda x:np.array([x[1]-.5,1.]),lambda x:sparse.csr_matrix([[0,1],[0,0]]),lambda x:bool(x[1]>=.5),(1,2),lambda x:True,temporary_held_columns=[0],max_iterations=2)
  self.assertTrue(r.success);self.assertEqual(r.x[0],0.)
 def test_hold_infeasible(self):
  r,m=solve(np.zeros(2),np.ones(2),lambda x:np.array([x[0]-.5,1.]),lambda x:sparse.csr_matrix([[1,0],[0,0]]),lambda x:False,(1,2),lambda x:True,temporary_held_columns=[0],max_iterations=1)
  self.assertFalse(r.success);np.testing.assert_array_equal(r.x,[0,0])
 def test_warm_cannot_rebase_hold(self):
  with self.assertRaises(ValueError):solve(np.zeros(2),np.ones(2),lambda x:np.array([1.,1.]),lambda x:sparse.csr_matrix((2,2)),lambda x:False,(1,2),lambda x:True,warm=np.array([.1,0]),temporary_held_columns=[0],max_iterations=1)
if __name__=='__main__':unittest.main()
