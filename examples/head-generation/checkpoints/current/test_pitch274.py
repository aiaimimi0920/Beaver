import unittest
from test_spatial_lashes196 import S,STYLE
class PitchTests(unittest.TestCase):
 def test_pitch_preserves_frontal_outline_and_root(self):
  old=STYLE['accent_sheet_slope']
  try:
   for side in [-1,1]:
    for t in [-.5,0,.5]:
     lo=S['curve'](t,1,STYLE['upper_lash_rails']);hi=S['curve'](t,2,STYLE['upper_lash_rails'])
     STYLE['accent_sheet_slope']=.45
     a=S['upper_accent_point'](side,t,lo,0);root=S['upper_accent_point'](side,t,hi,0)
     STYLE['accent_sheet_slope']=.8
     b=S['upper_accent_point'](side,t,lo,0);newroot=S['upper_accent_point'](side,t,hi,0)
     self.assertEqual(root,newroot);self.assertEqual(a[0],b[0]);self.assertEqual(a[2],b[2]);self.assertLess(b[1],a[1])
  finally:STYLE['accent_sheet_slope']=old
