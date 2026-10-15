import unittest
from test_spatial_lashes196 import S,STYLE
from test_style_math import pure_function

class ThinReturnTriangulationTests(unittest.TestCase):
 def test_actual_curved_tuft_front_remains_outward_on_both_sides(self):
  key='upper_swept_tuft_outline'
  sections=pure_function('eye_accents.py','outline_sections',{'STYLE':STYLE})[0](STYLE[key])
  for side in [-1,1]:
   vs,fs,zones=S['closed_ribbon_mesh'](sections,lambda t,r,d:S['upper_accent_point'](side,t,r,d,key),STYLE['accent_wing_thickness_m'],STYLE['accent_tuft_fold_m'],STYLE['accent_fold_ridge_fraction'],True)
   weighted=0
   for f,z in zip(fs,zones):
    if z not in (1,2):continue
    self.assertEqual(len(f),3)
    a,b,c=[vs[i] for i in f]
    weighted+=((b[2]-a[2])*(c[0]-a[0])-(b[0]-a[0])*(c[2]-a[2]))/2
   self.assertLess(weighted,0,'Positive volume must not conceal reversed front faces')
   self.assertGreater(S['signed_volume'](vs,fs),0)
