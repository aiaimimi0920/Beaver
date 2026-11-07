"""Test the actual source tongue geometry under both skin allocations."""
import ast
import math
from pathlib import Path
import unittest
from source_density_layout import adapt, tongue_budget


class InteriorBudgetTests(unittest.TestCase):
    def test_profiles_and_invalid_columns(self):
        self.assertEqual(tongue_budget({}), (16,9))
        self.assertEqual(tongue_budget({"sampling_detail_budget":1}), (12,5))
        self.assertEqual(tongue_budget({'sampling_detail_budget':1,'sampling_shell_columns':9}), (12,7))
        self.assertEqual(tongue_budget({'sampling_detail_budget':1,'sampling_shell_columns':13}), (12,5))
        with self.assertRaises(ValueError):
            tongue_budget({'sampling_detail_budget':1,'sampling_shell_columns':10})

    def test_source_geometry_is_closed_and_preserves_extents(self):
        root=Path(__file__).parents[1]/'checkpoints/current'
        sources={name:(root/name).read_text() for name in ('face.py','shell.py','mouth.py')}
        source=adapt(sources)['mouth.py']
        tree=ast.parse(source)
        fn=next(n for n in tree.body if isinstance(n,ast.FunctionDef) and n.name=='build_oral_cavity')
        start=next(i for i,n in enumerate(fn.body) if isinstance(n,ast.Assign) and any(isinstance(t,ast.Name) and t.id=='tv' for t in n.targets))
        body=ast.Module(body=fn.body[start:],type_ignores=[])
        snapshots=[]
        for columns,detail,expected in ((9,0,284),(9,1,164),(13,1,116)):
            captured={}
            def mesh(name,vertices,faces,color):
                captured.update(vertices=vertices,faces=faces)
                return None
            scope={'STYLE':{'sampling_shell_columns':columns,'sampling_detail_budget':detail},
                   'tongue_budget':tongue_budget,'pi':math.pi,'sin':math.sin,'cos':math.cos,
                   'coord':lambda x,y,z:(x,-z,y),'depth':lambda x,y:.05,'MOUTH_Y':1.58,
                   'mesh':mesh,'store_mouth_deltas':lambda *args:None}
            exec(compile(body,'<actual source tongue>','exec'),scope)
            vertices,faces=captured['vertices'],captured['faces']
            self.assertEqual(sum(len(face)-2 for face in faces),expected)
            uses={}
            for face in faces:
                for a,b in zip(face,face[1:]+face[:1]):
                    key=tuple(sorted((a,b)));uses[key]=uses.get(key,0)+1
            self.assertTrue(all(count==2 for count in uses.values()))
            self.assertTrue(all(math.isfinite(c) for v in vertices for c in v))
            snapshots.append([(min(v[i] for v in vertices),max(v[i] for v in vertices)) for i in range(3)])
        for bounds in snapshots[1:]:
            for old,new in zip(snapshots[0],bounds):
                self.assertAlmostEqual(old[0],new[0],places=12)
                self.assertAlmostEqual(old[1],new[1],places=12)


if __name__=='__main__':
    unittest.main()
