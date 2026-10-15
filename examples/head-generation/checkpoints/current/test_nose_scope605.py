import hashlib
import json,unittest
from pathlib import Path
from test_style_math import pure_function
class NoseScope605(unittest.TestCase):
 def test_only_nose_parameters_change(self):
  root=Path(__file__).parent
  current=json.loads((root/'face_style.json').read_text());base=json.loads((root/'baseline_fixtures.json').read_text())['602']['style']
  self.assertEqual(current["accent_lateral_branch_start"],.04);self.assertEqual(current["accent_lateral_branch_span"],.92)
  current["accent_lateral_branch_start"]=.16;current["accent_lateral_branch_span"]=.76
  current['preset']=base['preset'];current['calibration']['nose_stations']=base['calibration']['nose_stations']
  for key in ['outer_lash_wing_rails','accent_lateral_join_t','accent_lateral_inner_width_profile','accent_lateral_branch_width_H','accent_lateral_guide_inward_power','upper_swept_tuft_outline','lateral_accent_profile','accent_lateral_bottom_t','accent_lateral_center_profile','accent_lateral_branch_separation_H','upper_lash_rails', 'accent_main_lower_flank_drop_m', 'accent_main_lower_flank_recede_m', 'accent_final_crown_relief_m']: current[key]=base[key]
  current.pop('accent_lateral_front_bias',None)
  current.pop('accent_lateral_taper_anchor_profile',None)
  current.pop('accent_lateral_eye_rim_follow',None)
  self.assertEqual(current,base)

class NasalSmooth607(unittest.TestCase):
 def test_scope_symmetry_and_smooth_boundaries(self):
  from nose_refine import nasal_delta,bump
  for x in [.0,.005,.012,.021,.022,.025]:
   for h in [.15,.211,.25,.286,.32,.361,.4]:
    a=nasal_delta(x,h);b=nasal_delta(-x,h)
    self.assertAlmostEqual(a[0],-b[0]);self.assertAlmostEqual(a[1],b[1])
    if x>=.022 or h<=.211 or h>=.361:self.assertLess(abs(a[0])+abs(a[1]),1e-12)
  self.assertLess(bump(1-1e-5),1e-12)
 def test_monotonic_width_and_bounded_depth(self):
  from nose_refine import nasal_delta
  for h in [.211,.23,.25,.286,.32,.34,.361]:
   values=[]
   for i in range(441):
    x=-.022+i*.0001;dx,dy=nasal_delta(x,h)
    values.append(x+dx);self.assertLessEqual(abs(dy),.003+1e-12)
   self.assertTrue(all(b>a for a,b in zip(values,values[1:])))

class Silhouette609(unittest.TestCase):
 def test_apex_and_lip_scope(self):
  from nose_refine import silhouette_delta
  for h in [.1,.2,.221,.332,.4,.8]:
   self.assertAlmostEqual(silhouette_delta(0,h),0)
  self.assertAlmostEqual(silhouette_delta(0,.298),.0022)
  self.assertLess(silhouette_delta(0,.270),0)
 def test_lateral_symmetry(self):
  from nose_refine import silhouette_delta
  for h in [.23,.25,.27,.284,.298,.31]:
   for x in [0,.005,.01,.017,.02]:
    self.assertAlmostEqual(silhouette_delta(x,h),silhouette_delta(-x,h))
    if x>=.017:self.assertAlmostEqual(silhouette_delta(x,h),0)
