import unittest,math
from test_spatial_lashes196 import S,STYLE,C
class LateralParentTests(unittest.TestCase):
 def test_side_root_stays_below_main_fork_gap(self):
  for side in [-1,1]:
   for off in [-.006,0]:
    t=STYLE['accent_lateral_join_t']+off/C['eye_half_width']
    root=S['lateral_accent_point'](side,0,off,0)
    main=S['upper_accent_point'](side,t,S['curve'](t,1,STYLE['upper_lash_rails']),0)
    self.assertLess(root[2],main[2]-.001)
 def test_parent_change_preserves_lower_endpoint(self):
  for side in [-1,1]:
   x,y=S['lateral_position'](side,1,0)
   p=S['lateral_accent_point'](side,1,0,0)
   self.assertAlmostEqual(p[0],x,places=12)
   self.assertAlmostEqual(p[2],y,places=12)
