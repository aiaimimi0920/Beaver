"""Orientation feasibility is retained while initially invalid quality is restored."""
import unittest
import warnings
from unittest.mock import patch
import numpy as np
from scipy.sparse import csr_matrix
from forehead_solver_restore import restore_forehead_feasibility, fit_spacing_sparse
from forehead_fit_density import forehead_quantized_quality_pass
from orbital_support_rule import orbital_quad_quality


class FeasibilityRestoreTests(unittest.TestCase):
    def test_quality_can_improve_without_crossing_orientation_boundary(self):
        start = np.array([1.,0.])
        constraints = lambda p: np.array([p[0]-2,4-p[0],10.,p[0],p[0],p[0],p[0],.5])
        pattern = np.zeros((8,2)); pattern[:7,0] = 1
        observed = []
        with warnings.catch_warnings():
            warnings.filterwarnings('ignore',message='delta_grad == 0.0.*')
            result = restore_forehead_feasibility(start,np.array([2.,2.]),constraints,
                csr_matrix(pattern),(3,7),(7,8),lambda p: observed.append(p.copy()),100)
        self.assertTrue(observed)
        self.assertTrue(all(np.all(constraints(p)[3:7] > 0) for p in observed))
        self.assertGreaterEqual(result.x[0],2-1e-7)
        self.assertLessEqual(result.x[0],3)

    def test_roundoff_saturated_fields_are_not_forced_barrier_slacks(self):
        initial = np.array([-1., 1., 1., 1., 1., 1., 1., -1e-12, 0., 1e-12, .5])
        with patch('forehead_solver_restore.minimize') as minimize:
            restore_forehead_feasibility(np.zeros(2),np.ones(2),lambda p: initial,
                csr_matrix(np.ones((len(initial),2))),(3,7),(7,11),lambda p:False,10)
        constraint = minimize.call_args.kwargs['constraints'][0]
        np.testing.assert_array_equal(constraint.keep_feasible,
            [False,False,False,True,True,True,True,False,False,False,False])
        self.assertEqual(constraint.lb,0.)
        self.assertEqual(constraint.ub,np.inf)
        np.testing.assert_array_equal(constraint.fun(np.zeros(2)),initial)

    def test_saved_quality_rejects_rounding_at_short_edges(self):
        points = np.array([
            [0.0938652234267306, 0.0025097611999603245, 1.6585854902216397],
            [0.09379408599899339, 0.003654776472021028, 1.657784592763549],
            [0.09394540627574123, 0.0032900328467137226, 1.6588711459759404],
            [0.09391844606213799, 0.0028485462422897585, 1.6588275709500335]])
        quads = np.array([[0,1,2,3]])
        angles, ratios = orbital_quad_quality(points,quads)
        self.assertLess(angles.max(),150)
        self.assertGreater(angles.min(),25)
        self.assertLess(ratios.max(),7)
        self.assertFalse(forehead_quantized_quality_pass(points,quads))

    def test_sparse_spacing_keeps_all_constraints_feasible(self):
        constraints = lambda p: np.array([p[0],1.5-p[0]])
        seen = []
        with warnings.catch_warnings():
            warnings.filterwarnings('ignore',message='delta_grad == 0.0.*')
            result = fit_spacing_sparse(np.array([1.]),np.array([2.]),
                lambda p:(p[0]-2)**2,constraints,csr_matrix(np.ones((2,1))),
                lambda p:seen.append(p.copy()),100)
        self.assertTrue(seen)
        self.assertTrue(all(np.all(constraints(p)>=0) for p in seen))
        self.assertGreater(result.x[0],1.4)
        with self.assertRaises(ValueError):
            fit_spacing_sparse(np.array([2.]),np.array([2.]),lambda p:p[0]**2,
                constraints,csr_matrix(np.ones((2,1))),lambda p:None,1)

    def test_folded_initialization_is_rejected(self):
        with self.assertRaises(ValueError):
            restore_forehead_feasibility(np.zeros(2),np.ones(2),lambda p: -np.ones(8),
                csr_matrix(np.ones((8,2))),(3,7),(7,8),lambda p: None,5)


if __name__ == '__main__':
    unittest.main()
