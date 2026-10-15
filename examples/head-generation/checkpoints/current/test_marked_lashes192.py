import unittest
from test_style_math import STYLE,pure_function
curve,_=pure_function('face.py','curve',{})
class MarkedLashTests(unittest.TestCase):
 def test_outer_upper_sweep_rises_to_taper(self):
  r=STYLE['upper_lash_rails']
  self.assertGreater(r[-1][1],r[-2][1])
  self.assertGreater(curve(.75,2,r),curve(-.75,2,r))
 def test_side_band_turns_inward_at_lower_lid(self):
  p=STYLE['accent_lateral_center_profile']
  self.assertGreater(curve(.2,1,p),curve(.6,1,p))
  self.assertGreater(curve(.6,1,p),curve(.9,1,p))
  self.assertAlmostEqual(p[-1][1],STYLE['accent_lateral_bottom_t'])
  self.assertLess(p[-1][1],.8)  # 368 keeps lower endpoint on outer lower-eye arc
 def test_wing_has_lower_outward_fork(self):
  w=STYLE['outer_lash_wing_rails']
  self.assertGreater(max(p[0] for p in w),1.1)
  self.assertLess(max(p[0] for p in w),1.35)  # no excessive lateral horns
  self.assertLess(min(p[1] for p in w),.02)
  self.assertGreater(w[-1][1],w[-2][1])
