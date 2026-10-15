import unittest,math
from test_spatial_lashes196 import S,STYLE
class TuftSweepTests(unittest.TestCase):
 def test_inner_root_attaches_to_main_fold(self):
  t,r=min(STYLE['upper_swept_tuft_outline'],key=lambda p:p[0])
  for side in [-1,1]:
   p=S['upper_accent_point'](side,t,r,0,'upper_swept_tuft_outline');q=S['upper_accent_point'](side,t,r,0)
   self.assertAlmostEqual(p[1],q[1]-S['main_ridge_relief'](t,r)+STYLE['accent_tuft_root_cover_m'],places=12)
 def test_swept_leaf_centerline_is_continuous_and_mirrored(self):
  start=min(STYLE['upper_swept_tuft_outline'],key=lambda p:p[0]);end=max(STYLE['upper_swept_tuft_outline'],key=lambda p:p[0]);last=None
  for i in range(101):
   u=i/100;t=start[0]*(1-u)+end[0]*u;r=start[1]*(1-u)+end[1]*u
   p=S['upper_accent_point'](1,t,r,0,'upper_swept_tuft_outline');q=S['upper_accent_point'](-1,t,r,0,'upper_swept_tuft_outline')
   self.assertTrue(all(math.isfinite(x) for x in p));self.assertAlmostEqual(p[0],-q[0]);self.assertEqual(p[1:],q[1:])
   if last:self.assertLess(math.dist(last,p),.001)
   last=p
