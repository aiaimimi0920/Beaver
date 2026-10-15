import unittest,math
from ear_boundary import calibrate_opening_sweep
class Opening541(unittest.TestCase):
 def test_scope_bound_symmetry(self):
  p=[(0,0,-.5),(0,0,.5),(.2,.10,-.3),(-.2,.10,-.3),(.2,.10,-.42),(.2,.10,-.1)]
  q=calibrate_opening_sweep(p,p,{2,3,4,5},dict(origin_y_m=0,height_m=1,ear_post_return_width_m=.0105,ear_post_lower_return_width_m=.007,ear_post_return_depth_m=.002))
  self.assertEqual(q[:2],p[:2]);self.assertEqual(q[4:],p[4:]);self.assertEqual(q[2][2],p[2][2]);self.assertAlmostEqual(math.dist(q[2],p[2]),.004);self.assertAlmostEqual(q[2][0],-q[3][0]);self.assertAlmostEqual(q[2][1],q[3][1])
