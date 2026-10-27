"""Source upper-ear refinement stays bounded and does not regress the root patch."""
import copy
import json
from pathlib import Path
import unittest
import numpy as np

HERE = Path(__file__).resolve().parent


class EarFrontRefinementTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.fixture = json.loads((HERE/'ear_front_refinement_fixture.json').read_text())
        cls.scope = {}
        for name in ('source_ear_quad_solver.py','ear_front_refinement.py'):
            exec(compile((HERE/name).read_text(),name,'exec'),cls.scope)
        cls.results = []
        for side in (-1,1):
            item = copy.deepcopy(cls.fixture)
            for point in item['original_vertices']+item['layout']['vertices']:
                point[0] = side*abs(point[0])
            before = copy.deepcopy(item['layout'])
            result = cls.scope['flow_refine_upper_front'](item['layout'],item['original_vertices'])
            cls.results.append((item['original_vertices'],before,result))

    def test_root_attachment_topology_and_attribute_lineage_stay_exact(self):
        for _,before,result in self.results:
            self.assertEqual(result['quads'],before['quads'])
            self.assertEqual(result['authored_vertex_weights'],before['authored_vertex_weights'])
            self.assertEqual(result['fixed_vertex_ids'],[i for i in before['fixed_vertex_ids'] if i!=26])
            fixed = set(result['fixed_vertex_ids'])
            fixed.update(i for q in before['quads'][31:34] for i in q)
            for i in fixed:
                self.assertEqual(result['vertices'][i],before['vertices'][i])
            changed = {i for i,(a,b) in enumerate(zip(before['vertices'],result['vertices'])) if a!=b}
            self.assertLessEqual(changed,{18,19,22,23,24,25,35,26})

    def test_explicit_outer_support_and_existing_source_limits(self):
        for original,before,result in self.results:
            original = np.asarray(original)
            p = np.asarray(result['vertices'])
            self.assertLessEqual(np.linalg.norm(p[26]-original[26]),.001)
            self.assertLessEqual(np.linalg.norm(p[:40]-original,axis=1).max(),
                                 result['upper_front_refinement']['source_bound_m']+1e-12)
            self.assertLessEqual(np.linalg.norm(p[42]-p[[37,36,39,27]].mean(0)),.001)
            report = result['source_continuity']
            self.assertGreaterEqual(report['upper_cap_normal_dot'],.65)
            self.assertLessEqual(report['upper_front_review_max_corner_degrees'],167)
            self.assertGreaterEqual(result['min_diagonal_pair_dot'],.49)
            self.assertTrue(np.isfinite(p).all())

    def test_bad_dimensions_anchors_and_nonfinite_inputs_fail_closed(self):
        fn = self.scope['flow_refine_upper_front']
        base = self.fixture
        with self.assertRaises(ValueError):
            fn(copy.deepcopy(base['layout']),base['original_vertices'][:-1])
        broken = copy.deepcopy(base['layout'])
        broken['fixed_vertex_ids'].remove(27)
        with self.assertRaises(ValueError):
            fn(broken,base['original_vertices'])
        broken = copy.deepcopy(base['layout'])
        broken['vertices'][0][1] = float('nan')
        with self.assertRaises(ValueError):
            fn(broken,base['original_vertices'])


if __name__ == '__main__':
    unittest.main()
