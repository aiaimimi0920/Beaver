import unittest,math
from ear_boundary import coherent_bridge_targets
class Bridge545(unittest.TestCase):
 def test_rigid_local_translation(self):
  p=[(0,0,0),(1,0,0),(0,1,0),(4,4,4)];q=[(0,.1,0),(1,.3,0),(0,1.2,0),(4,4,4)];r=coherent_bridge_targets(p,q,[[0,1,2]])
  self.assertEqual(r[3],p[3])
  for a,b in [(0,1),(1,2),(2,0)]:self.assertAlmostEqual(math.dist(r[a],r[b]),math.dist(p[a],p[b]))
