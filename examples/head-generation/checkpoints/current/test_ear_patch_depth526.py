import unittest
from ear_boundary import fair_interior_depth
class EarPatch526(unittest.TestCase):
 def test_planar_depth_remains_fixed(self):
  p=[(0,0,0),(1,0,0),(2,0,0)]
  self.assertEqual(fair_interior_depth(p,{1:{0,2}},{1}),p)
 def test_only_interior_depth_changes_and_is_bounded(self):
  p=[(0,0,0),(1,.02,1),(2,0,2)];q=fair_interior_depth(p,{1:{0,2}},{1})
  self.assertEqual(q[0],p[0]);self.assertEqual(q[2],p[2])
  self.assertEqual(q[1][0],p[1][0]);self.assertEqual(q[1][2],p[1][2])
  self.assertAlmostEqual(q[1][1],.017)
 def test_empty_scope_is_identity(self):
  p=[(0,.02,0),(1,0,0)];self.assertEqual(fair_interior_depth(p,{0:{1}},set()),p)
