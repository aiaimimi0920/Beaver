"""Pure geometry contracts for budget ear transition, not native acceptance."""
import ast,math,unittest
from pathlib import Path

scope={}
tree=ast.parse(Path(__file__).with_name('ears.py').read_text())
for name in ('source_triangle_score','source_quad_split','source_row_transition'):
    node=next(n for n in tree.body if isinstance(n,ast.FunctionDef) and n.name==name)
    exec(compile(ast.Module(body=[node],type_ignores=[]),'source-ear-contract','exec'),scope)

class BudgetEarTransitionTests(unittest.TestCase):
    def test_transition_covers_strip_without_extra_vertices(self):
        vertices=[(x,0.,0.) for x in (0.,.5,1.)]+[(x,.5,0.) for x in (0.,.25,.5,.75,1.)]
        faces=scope['source_row_transition']([0,1,2],[3,4,5,6,7],vertices)
        self.assertEqual(len(faces),6)
        self.assertEqual(set(i for f in faces for i in f),set(range(8)))
        areas=[];edges={}
        for face in faces:
            a,b,c=[vertices[i] for i in face]
            areas.append(((b[0]-a[0])*(c[1]-a[1])-(b[1]-a[1])*(c[0]-a[0]))/2)
            self.assertGreaterEqual(scope['source_triangle_score']([a,b,c]),.35)
            for a,b in zip(face,face[1:]+face[:1]):
                key=tuple(sorted((a,b)));edges[key]=edges.get(key,0)+1
        self.assertTrue(all(a>0 for a in areas));self.assertAlmostEqual(sum(areas),.5)
        self.assertTrue(all(n in (1,2) for n in edges.values()))
        self.assertEqual(sum(n==1 for n in edges.values()),8)
    def test_degenerate_and_impossible_geometry_rejected(self):
        self.assertEqual(scope['source_triangle_score']([(0,0,0)]*3),0)
        self.assertEqual(scope['source_triangle_score']([(float('nan'),0,0),(1,0,0),(0,1,0)]),0)
        with self.assertRaises(ValueError):scope['source_row_transition']([0,1],[2,3],[(0,0,0)]*4)
    def test_scores_are_mirror_invariant(self):
        p=[(0,0,0),(.004,0,0),(.001,.004,0)]
        self.assertAlmostEqual(scope['source_triangle_score'](p),scope['source_triangle_score']([(-x,y,z) for x,y,z in p]))

if __name__=='__main__':unittest.main()
