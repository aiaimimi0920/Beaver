import unittest
from collections import Counter
from main_curve import curved_section,curved_mesh
class MainCurve390(unittest.TestCase):
 def test_boundaries_crest_and_thickness(self):
  low=(0,0,0);crest=(0,-.003,.005);high=(0,0,.01)
  r=curved_section(low,crest,high,.00018)
  self.assertEqual(r[1],low);self.assertEqual(r[3],crest);self.assertEqual(r[5],high)
  self.assertAlmostEqual(r[2][1],-.00225)
  for a,b in [(1,0),(2,9),(3,8),(4,7),(5,6)]:
   self.assertAlmostEqual(r[b][1]-r[a][1],.00018)
 def test_closed_mesh_and_front_lanes(self):
  columns=[[(0,0,.005)],curved_section((.01,0,0),(.01,-.003,.005),(.01,0,.01),.00018),[(.02,0,.005)]]
  v,f,z=curved_mesh(columns)
  edges=Counter(tuple(sorted(e)) for p in f for e in zip(p,p[1:]+p[:1]))
  self.assertTrue(all(n==2 for n in edges.values()));self.assertEqual(len(f),len(z));self.assertEqual(sum(x in (1,2) for x in z),8)
