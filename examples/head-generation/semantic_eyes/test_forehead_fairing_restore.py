"""Small synthetic feasibility and state-isolation tests; no facial solve."""
import unittest
from unittest.mock import patch
from types import SimpleNamespace
import numpy as np
from scipy.sparse import csr_matrix
from forehead_fairing_restore import forehead_restore_fairing_iterate, forehead_minimum_improvement_passes

class ForeheadFairingRestoreTests(unittest.TestCase):
    def test_tiny_inner_violation_is_restored_not_relaxed(self):
        warm=np.array([-.000475]);anchor=np.array([0.]);radius=np.array([1.]);seen=[]
        def constraints(x):return np.array([1., x[0]])
        def remember(x):
            seen.append(x.copy());return bool(np.min(constraints(x))>=0)
        solved,meta=forehead_restore_fairing_iterate(warm,anchor,radius,constraints,
            csr_matrix([[0],[1]]),(0,1),remember,jacobian=lambda x:csr_matrix([[0.],[1.]]))
        self.assertTrue(meta['callback_selected_feasible']);self.assertGreaterEqual(solved.x[0],0.)
        self.assertEqual(warm[0],-.000475);self.assertEqual(anchor[0],0.);self.assertFalse(meta['full_model_success'])
    def test_original_bounds_not_recentered_and_only_orientation_kept(self):
        def fake(fun,x0,**kw):
            np.testing.assert_array_equal(kw['bounds'].lb,[-1.]);np.testing.assert_array_equal(kw['bounds'].ub,[1.])
            hard=kw['constraints'][0];np.testing.assert_array_equal(hard.keep_feasible,[True,False])
            np.testing.assert_array_equal(hard.lb,0.);np.testing.assert_array_equal(hard.fun(x0),[1.,-.1])
            return SimpleNamespace(x=x0.copy(),success=True)
        with patch('forehead_fairing_restore.minimize',side_effect=fake):
            result,meta=forehead_restore_fairing_iterate(np.array([.9]),np.array([0.]),np.array([1.]),
                lambda x:np.array([1.,-.1]),csr_matrix([[0],[1]]),(0,1),lambda x:False)
        self.assertFalse(meta['callback_selected_feasible']);self.assertFalse(meta['full_model_success'])
    def test_folded_or_outside_or_invalid_warm_rejected(self):
        for warm,rows in [(1.1,[1.,0.]),(0.,[0.,0.]),(0.,[float('nan'),0.])]:
            with patch('forehead_fairing_restore.minimize')as solve:
                with self.assertRaises(ValueError):forehead_restore_fairing_iterate(np.array([warm]),np.array([0.]),np.array([1.]),lambda x:np.array(rows),None,(0,1),lambda x:False)
                solve.assert_not_called()
    def test_positive_minimum_is_strict_admission(self):
        self.assertTrue(forehead_minimum_improvement_passes(2., 1.8, .1))
        self.assertFalse(forehead_minimum_improvement_passes(2., 1.8000000001, .1))
        self.assertFalse(forehead_minimum_improvement_passes(2., 1.99, .1))
        self.assertFalse(forehead_minimum_improvement_passes(2., float('nan'), .1))
    def test_iteration_budget_is_bounded(self):
        with self.assertRaises(ValueError):forehead_restore_fairing_iterate([0.],[0.],[1.],lambda x:np.array([1.]),None,(0,1),lambda x:False,max_iterations=121)

if __name__=='__main__':unittest.main()
