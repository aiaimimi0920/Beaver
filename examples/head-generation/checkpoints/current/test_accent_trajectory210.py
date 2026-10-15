import unittest
from test_spatial_lashes196 import S,STYLE,C

class AccentTrajectoryTests(unittest.TestCase):
 def test_lash_slope_is_independent_of_accepted_eye_slope(self):
  scale=STYLE['accent_trajectory_slope_scale']
  self.assertGreater(scale,0)
  self.assertLess(scale,1)
  eye_slope=C['eye_corner_slope']
  for side in [-1,1]:
   for t in [-.8,0,.8]:
    rise=S['curve'](t,2,STYLE['upper_lash_rails'])
    q=S['upper_accent_point'](side,t,rise,0)
    expected=S['Y0']+S['H']*(C['eye_corner_y']+eye_slope*t*scale+rise)
    self.assertAlmostEqual(q[2],expected)
  self.assertEqual(C['eye_corner_slope'],eye_slope)
