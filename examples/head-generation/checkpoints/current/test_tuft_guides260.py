import unittest, math
from test_spatial_lashes196 import S, STYLE
from test_style_math import pure_function

class TuftSpatialGuideTests(unittest.TestCase):
 def test_broad_root_follows_parent_and_tip_converges(self):
  key='upper_swept_tuft_outline'
  rows=pure_function('eye_accents.py','outline_sections',{'STYLE':STYLE})[0](STYLE[key])
  for side in [-1,1]:
   gaps=[]
   for t,lo,hi in rows:
    lower=S['upper_accent_point'](side,t,lo,0,key)
    upper=S['upper_accent_point'](side,t,hi,0,key)
    gaps.append(lower[1]-upper[1])
    self.assertLess(abs(gaps[-1]),.012)
    self.assertGreaterEqual(upper[2],lower[2])
    self.assertTrue(all(math.isfinite(x) for x in (*lower,*upper)))
   t,lo,hi=rows[0]
   for rise in (lo,hi):
    root=S['upper_accent_point'](side,t,rise,0,key)
    parent=S['upper_accent_point'](side,t,rise,0)
    self.assertAlmostEqual(root[1],parent[1]-S['main_ridge_relief'](t,rise)+STYLE['accent_tuft_root_cover_m'],places=12)
   self.assertLess(max(abs(b-a) for a,b in zip(gaps,gaps[1:])),.005)
   self.assertAlmostEqual(gaps[-1],0,places=12)
   self.assertGreater(max(abs(g) for g in gaps),.001)  # broad root inherits parent tilt
 def test_removed_twist_controls_are_not_advertised(self):
  self.assertNotIn('accent_tuft_cross_twist_m',STYLE)
  self.assertNotIn('accent_tuft_mid_forward_m',STYLE)
