import unittest,json
from pathlib import Path
from lateral_fit import eye_rim_x_at_height,eye_rim_follow_shift
C=json.loads(Path(__file__).with_name('face_style.json').read_text())['calibration']
class EyeRimTests(unittest.TestCase):
 def test_lower_rim_inversion(self):
  for t in [.5,.7,.8,.9,.95,.99]:
   z=C['origin_y_m']+C['height_m']*(C['eye_corner_y']+C['eye_corner_slope']*t-C['eye_lower_arch']*(1-t**C['eye_lower_horizontal_power'])**C['eye_lower_power'])
   expected=C['height_m']*(C['eye_center_x']+C['eye_half_width']*t)
   self.assertAlmostEqual(eye_rim_x_at_height(z,C),expected,places=10)
 def test_root_preserved_and_mirror_symmetry(self):
  self.assertEqual(eye_rim_follow_shift(.079,1.65,1,0,C),0)
  a=eye_rim_follow_shift(.078,1.645,1,.5,C);b=eye_rim_follow_shift(-.078,1.645,-1,.5,C);self.assertAlmostEqual(a,-b)
 def test_large_drift_rejected(self):
  with self.assertRaises(AssertionError):eye_rim_follow_shift(.1,1.64,1,.5,C)

 def test_contour_follow_preserves_both_existing_joins(self):
  ends=((.074,1.654),(.065,1.632))
  for f,(x,z) in [(0,ends[0]),(1,ends[1])]:
   self.assertAlmostEqual(eye_rim_follow_shift(x,z,1,f,C,ends),0,places=12)

 def test_descending_rim_does_not_bulge_outward(self):
  ends=((.074,1.654),(.065,1.632));values=[]
  for i in range(101):
   f=i/100;x=ends[0][0]*(1-f)+ends[1][0]*f;z=ends[0][1]*(1-f)+ends[1][1]*f
   values.append(x+eye_rim_follow_shift(x,z,1,f,C,ends))
  self.assertTrue(all(b<a for a,b in zip(values,values[1:])))
