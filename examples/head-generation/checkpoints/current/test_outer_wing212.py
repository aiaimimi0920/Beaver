import unittest
from test_spatial_lashes196 import S,STYLE
class OuterWingTests(unittest.TestCase):
 def test_local_skin_blend_only_repositions_secondary_wing(self):
  blend=STYLE['accent_outer_root_skin_blend']
  try:
   for t in [.8,1,1.2]:
    h=S['curve'](t,1,STYLE['outer_lash_wing_rails'])
    main=S['upper_accent_point'](1,t,h,0)
    STYLE['accent_outer_root_skin_blend']=0
    a=S['upper_accent_point'](1,t,h,0,'outer_lash_wing_rails')
    STYLE['accent_outer_root_skin_blend']=blend
    b=S['upper_accent_point'](1,t,h,0,'outer_lash_wing_rails')
    self.assertAlmostEqual(a[0],b[0]);self.assertAlmostEqual(a[2],b[2])
    self.assertGreaterEqual(b[1]-a[1],-1e-12)
    self.assertLessEqual(b[1]-a[1],STYLE['accent_outer_root_skin_limit_m']*blend+1e-12)
    self.assertEqual(main,S['upper_accent_point'](1,t,h,0))
  finally:STYLE['accent_outer_root_skin_blend']=blend
