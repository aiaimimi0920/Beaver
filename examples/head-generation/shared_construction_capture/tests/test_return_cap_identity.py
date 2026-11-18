"""Source boundary/identity adversaries and exact original cap numeric smoke test."""
from pathlib import Path
import sys
import unittest
HERE=Path(__file__).resolve().parent
sys.path.insert(0,str(HERE.parent))
from cap_observer import directed_boundary,validate_allocation
from joint_source_fixture import source_pair


class CapTests(unittest.TestCase):
    def test_directed_boundary_and_cyclic_allocation(self):
        self.assertEqual(directed_boundary([['a','b','c'],['a','c','d']]),['a','b','c','d'])
        boundary,moving=validate_allocation([['a','b','c'],['a','c','d']],['c','d','a','b'],{'a','b'})
        self.assertEqual(set(moving),{'c','d'})

    def test_source_winding_rejected(self):
        with self.assertRaises(ValueError):directed_boundary([['a','b','c'],['a','d','c']])

    def test_source_nonmanifold_or_unrelated_pair(self):
        for triangles in ([['a','b','c'],['d','e','f']],[['a','b','c'],['a','b','c']]):
            with self.assertRaises(ValueError):directed_boundary(triangles)

    def test_allocation_wrong_boundary_winding_or_ownership(self):
        for output,original in [(['a','d','c','b'],{'a','b'}),(['a','b','c','x'],{'a','b'}),
                                (['a','b','c','d'],{'a','b','c'})]:
            with self.assertRaises(ValueError):validate_allocation([['a','b','c'],['a','c','d']],output,original)

    def test_exact_original_solver_and_stored_quantization(self):
        import numpy as np
        ns={};original,_=source_pair();exec(compile(original,'original_cap','exec'),ns)
        vertices=np.array([[3,0,0],[3.01,0,0],[3,.01,0],[3.01,.01,0],
                           [3.01,.005,0],[3,.005,0]],dtype=np.float32).astype(float)
        faces=[[0,1,4],[0,4,5],[5,4,3,2]]
        result=ns['solve_return_cap_with_seed'](vertices,faces,[0,1],set(range(4)),ns['ear_quad_least_squares'])
        after=vertices.copy();after[result['boundary_global_ids']]=np.asarray(result['vertices'],np.float32)
        ns['check_stored_cap_neighbors'](vertices,after,faces,result)
        stored=after[result['boundary_global_ids']]
        normal,angle,area=ns['ear_quad_measures']((stored-stored[0])*1000,[[0,1,2,3]])
        self.assertGreaterEqual(normal.min(),.49)
        self.assertGreaterEqual(angle.min(),result['old_min_angle']-1e-5)
        self.assertGreater(area.min(),1e-8)
        self.assertTrue(np.array_equal(after[:4],vertices[:4]))
        self.assertLessEqual(np.linalg.norm(after-vertices,axis=1).max(),.0005)

    def test_observer_has_before_mutation_and_before_deletion_hooks(self):
        original,observed=source_pair()
        self.assertIn("        _observe_cap('before',locals())\n        for i,point",observed)
        self.assertIn('        _observe_cap("allocated",locals())\n        bm.faces.remove(first)',observed)
        self.assertNotIn('_observe_cap',original)


if __name__=='__main__':unittest.main()
