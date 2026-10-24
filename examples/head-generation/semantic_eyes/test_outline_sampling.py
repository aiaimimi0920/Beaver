import unittest
from source_outline_sampling import refined_outline

class OutlineTests(unittest.TestCase):
    def test_embedded_helper_has_its_own_math_dependencies(self):
        import inspect
        scope={};exec(inspect.getsource(refined_outline),scope)
        points,faces,_=scope['refined_outline']([(0,0,0),(1,0,0),(1,1,0),(0,1,0)],[(0,1,2,3)],[1,2,3])
        self.assertEqual(len(points),6)

    def test_source_vertices_preserved_and_edges_conform(self):
        v=[(0,0,0),(1,0,0),(1,1,0),(0,1,0)];f=[(0,1,2,3)]
        points,faces,report=refined_outline(v,f,[1,2,3],.1)
        self.assertEqual(points[:4],v);self.assertEqual(f,[(0,1,2,3)])
        self.assertEqual(len(points),6);self.assertEqual(len(faces),1)
        self.assertEqual(faces[0],(0,1,4,2,5,3))
        self.assertTrue(report['existing_vertices_unchanged'])
        self.assertLessEqual(report['maximum_curve_deviation_m'],.1+1e-12)
    def test_interior_edge_rejected(self):
        with self.assertRaises(ValueError):refined_outline([(0,0,0),(1,0,0),(1,1,0),(0,1,0)],[(0,1,2),(0,2,3)],[1,0,2])
    def test_mirror_invariant(self):
        v=[(0,0,0),(1,0,0),(1,1,0),(0,1,0)];f=[(0,1,2,3)]
        a,fa,_=refined_outline(v,f,[1,2,3]);b,fb,_=refined_outline([(-x,y,z) for x,y,z in v],f,[1,2,3]);self.assertEqual(fa,fb)
        for p,q in zip(a,b):self.assertEqual((-p[0],p[1],p[2]),q)
    def test_invalid_budget_or_chain_rejected(self):
        v=[(0,0,0),(1,0,0),(1,1,0)];f=[(0,1,2)]
        for chain,budget in [([0,1,0],.002),([0,1,2],-.1),([0,1,2],float('nan'))]:
            with self.assertRaises(ValueError):refined_outline(v,f,chain,budget)

if __name__=='__main__':unittest.main()
