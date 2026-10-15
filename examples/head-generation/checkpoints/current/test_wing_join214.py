import unittest,math
from test_spatial_lashes196 import S,STYLE
class WingJoinTests(unittest.TestCase):
 def test_secondary_wing_root_is_exactly_on_main_strip(self):
  t,lo,hi=STYLE['outer_lash_wing_rails'][0]
  self.assertEqual(lo,hi)
  self.assertAlmostEqual(lo,S['curve'](t,1,STYLE['upper_lash_rails']),places=9)
  for side in [-1,1]:
   parent=S['upper_accent_point'](side,t,lo,0)
   wing=S['upper_accent_point'](side,t,lo,0,'outer_lash_wing_rails')
   self.assertLess(math.dist(parent,wing),1e-12)
 def test_tuft_fold_is_relief_not_wall_thickness(self):
  self.assertLess(STYLE['accent_wing_thickness_m'],.0003)
  self.assertGreater(STYLE['accent_tuft_fold_m'],STYLE['accent_wing_thickness_m']*3)
  self.assertLess(STYLE['accent_tuft_fold_m'],.003)
