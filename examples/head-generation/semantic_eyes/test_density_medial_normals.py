"""Exercise both generated normal grouping sites without a Blender dependency."""
import ast
from pathlib import Path
from types import SimpleNamespace as N
import unittest
from source_density_layout import adapt


class DensityMedialTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        root=Path(__file__).parents[1]/'checkpoints/current'
        sources={p.name:p.read_text() for p in root.glob('*.py')}
        source=adapt(sources)['pipeline.py']
        tree=ast.parse(source)
        cls.sites=[node for node in ast.walk(tree) if isinstance(node,ast.Assign)
                   and any(isinstance(t,ast.Name) and t.id=='side' for t in node.targets)]
        assert len(cls.sites)==2

    def check_side(self,style,region,x,poly_x,expected):
        for site in self.sites:
            scope={'STYLE':style,'region':region,'index':0,
                   'data':N(vertices=[N(co=N(x=x))]),'poly':N(center=N(x=poly_x))}
            exec(compile(ast.Module(body=[site],type_ignores=[]),'<normal grouping>','exec'),scope)
            self.assertEqual(scope['side'],expected)

    def test_both_sites_keep_legacy_and_non_skin_partitions(self):
        for style in ({},{'sampling_detail_budget':0},
                      {'sampling_detail_budget':1,'sampling_shell_columns':9}):
            for region in (1,2,7):
                for side in (-1,1):self.check_side(style,region,0,side,side)
        for region in (7,1001,2001):
            self.check_side({'sampling_detail_budget':1},region,0,-1,-1)

    def test_explicit_and_omitted_default_thirteen_are_continuous(self):
        for style in ({'sampling_detail_budget':1},
                      {'sampling_detail_budget':1,'sampling_shell_columns':13}):
            for region in (1,2):
                for side in (-1,1):self.check_side(style,region,0,side,0)

    def test_off_center_vertices_unchanged(self):
        for style in ({},{'sampling_detail_budget':1}):
            for region in (1,2,7):self.check_side(style,region,.001,-1,0)


if __name__=='__main__':unittest.main()
