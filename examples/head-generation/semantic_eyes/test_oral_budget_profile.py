"""Test generated oral sampling and skin-only accumulation profile boundaries."""
import ast
from pathlib import Path
from types import SimpleNamespace as N
import unittest
from source_density_layout import adapt, oral_ring_fractions


class OralBudgetProfileTests(unittest.TestCase):
    def test_default_and_nine_column_geometry_stays_unchanged(self):
        for style in ({},{'sampling_detail_budget':0},
                      {'sampling_detail_budget':1,'sampling_shell_columns':9}):
            self.assertEqual(oral_ring_fractions('mouth',style),[0,.30])
            self.assertEqual(oral_ring_fractions('eye',style),[0,.30,.65])

    def test_thirteen_column_profile_spends_one_local_ring(self):
        for style in ({'sampling_detail_budget':1},
                      {'sampling_detail_budget':1,'sampling_shell_columns':13}):
            result=oral_ring_fractions('mouth',style)
            self.assertEqual(result,[0,.50,.75])
            self.assertEqual(result[0],0)
            self.assertTrue(all(a<b for a,b in zip(result,result[1:])))
            self.assertLess(result[-1],1)  # Original outer boundary is appended.
            self.assertEqual(oral_ring_fractions('eye',style),[0,.30,.65])
        with self.assertRaises(ValueError):oral_ring_fractions('unknown',{})

    def test_actual_generated_accumulator_excludes_only_non_skin_in_opt_in(self):
        root=Path(__file__).parents[1]/'checkpoints/current'
        source=adapt({p.name:p.read_text() for p in root.glob('*.py')})['pipeline.py']
        tree=ast.parse(source)
        matches=[node for node in ast.walk(tree) if isinstance(node,ast.If)
                 and len(node.body)==1 and isinstance(node.body[0],ast.AugAssign)
                 and isinstance(node.body[0].target,ast.Subscript)
                 and isinstance(node.body[0].target.value,ast.Name)
                 and node.body[0].target.value.id=='normals']
        self.assertEqual(len(matches),1)
        for style,active in (({},False),({'sampling_detail_budget':1,'sampling_shell_columns':9},False),
                             ({'sampling_detail_budget':1},True),
                             ({'sampling_detail_budget':1,'sampling_shell_columns':13},True)):
            for region in (1,2,3,4,1001):
                scope={'STYLE':style,'region':region,'index':0,'normals':[0.],
                       'poly':N(normal=2.,area=3.)}
                exec(compile(ast.Module(body=[matches[0]],type_ignores=[]),'<actual accumulator>','exec'),scope)
                self.assertEqual(scope['normals'][0],6. if not active or region in (1,2) else 0.)


if __name__=='__main__':unittest.main()
