import unittest
from lateral_fit import upper_wing_depth_blend
class WingDepth(unittest.TestCase):
 def test_anchors_and_overlap(self):
  self.assertEqual(upper_wing_depth_blend(.002,0),0)
  self.assertEqual(upper_wing_depth_blend(.002,1),0)
  self.assertAlmostEqual(upper_wing_depth_blend(.002,.5),.002)
 def test_bounded(self):
  for i in range(101):self.assertLessEqual(abs(upper_wing_depth_blend(-.002,i/100)),.002)
