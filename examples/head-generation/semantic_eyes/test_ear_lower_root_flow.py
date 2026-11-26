"""Source lower-root correction stays bounded and preserves authored ownership."""
import json
from pathlib import Path
import unittest
import numpy as np

HERE = Path(__file__).resolve().parent


class LowerRootTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.scope = {}
        for name in ('source_ear_quad_solver.py','ear_lower_root_flow.py'):
            exec(compile((HERE/name).read_text(),name,'exec'),cls.scope)
        cls.original = json.loads((HERE/'ear_front_refinement_fixture.json').read_text())['original_vertices']
        cls.core = json.loads((HERE/'ear_attachment_fixture.json').read_text())
        cls.fixed = [0,1,5,9,15,21,27,38,39,28,30,32,34,36,4,8,14,20,26]

    def fixture(self, side):
        original = np.asarray(self.original,dtype=float)
        vertices = np.asarray(self.core['vertices'],dtype=float)
        original[:,0] = side*np.abs(original[:,0])
        vertices[:,0] = side*np.abs(vertices[:,0])
        unit = np.linalg.norm(original[27]-original[0])/56
        return (vertices-original[0])/unit, (original-original[0])/unit, original, unit

    def test_bilateral_source_roles_and_bounds(self):
        outputs = []
        for side in (-1,1):
            points,base,original,unit = self.fixture(side)
            result, report = self.scope['flow_lower_root_stations'](
                points,base,original,unit,self.fixed,self.core['quads'])
            changed = np.flatnonzero(np.any(result != points,axis=1)).tolist()
            self.assertEqual(changed,[2,3])
            self.assertTrue(np.array_equal(result[self.fixed],points[self.fixed]))
            self.assertLess(report['max_increment_mm'],1)
            self.assertGreater(report['min_pair_dot_after'],report['min_pair_dot_before'])
            self.assertLessEqual(report['chain_turn_after'],80.1)
            self.assertLess(np.linalg.norm(result-base,axis=1).max(),3.999999)
            outputs.append((result-points)*[side,1,1])
        self.assertTrue(np.allclose(*outputs,atol=1e-12))

    def test_authoring_recipe_retains_helper_dependency(self):
        from feature_flow_recipe import retained_sources
        self.assertEqual(retained_sources()['ear_lower_root_flow.py'],
                         (HERE/'ear_lower_root_flow.py').read_text())

    def test_source_contract_and_budget_fail_closed(self):
        points,base,original,unit = self.fixture(1)
        call = self.scope['flow_lower_root_stations']
        with self.assertRaises(ValueError):
            call(points,base,original,unit,self.fixed+[2],self.core['quads'])
        bad=points.copy();bad[13]=base[13]+[5,0,0]
        with self.assertRaises(ValueError):
            call(bad,base,original,unit,self.fixed,self.core['quads'])


if __name__ == '__main__':
    unittest.main()
