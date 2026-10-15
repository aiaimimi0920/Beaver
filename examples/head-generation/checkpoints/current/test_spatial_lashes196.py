import math,unittest
from test_style_math import pure_function,STYLE
C=STYLE['calibration'];S={'STYLE':STYLE,'C':C,'H':C['height_m'],'Y0':C['origin_y_m'],'Z0':C['chin_depth_m']}
for n in ['curve','width','nose_profile_factor','nose','lip_shoulder_support','lip_detail_depth','base_depth','rim_depth','depth','coord','eye_contour']:
 S[n]=pure_function('face.py',n,S)[0]
for n in ['upper_band_bow','main_band_roll','upper_lid_support','upper_attached_depth','main_ridge_relief','tuft_rear_scoop','tuft_free_bow','tuft_fold_relief','upper_accent_point','lateral_position','lateral_attached_depth','lateral_accent_point','signed_volume','closed_ribbon_mesh']:
 S[n]=pure_function('eye_accents.py',n,S)[0]

class SpatialLashTests(unittest.TestCase):
 def test_lateral_top_uses_exact_wing_lower_rail_in_three_dimensions(self):
  for side in [-1,1]:
   for off in [-.012,-.006,0]:
    t=STYLE['accent_lateral_join_t']+off/C['eye_half_width']
    expected=S['upper_accent_point'](side,t,S['curve'](t,1,STYLE['outer_lash_wing_rails']),0,'outer_lash_wing_rails')
    actual=S['lateral_accent_point'](side,0,off,0)
    self.assertLess(math.dist(actual,expected),1e-12)
 def test_thin_return_follows_fold_ridge_instead_of_filling_a_solid_wedge(self):
  vs,fs,z=S['closed_ribbon_mesh']([[-1,0,.025],[0,0,.025],[1,0,.025]],lambda t,h,d:(t*.02,-d,h*.236),.00018,.0032,.78,True)
  for start in [0,6,12]:
   for a,b in [(0,1),(2,5),(3,4)]:
    self.assertAlmostEqual(math.dist(vs[start+a],vs[start+b]),.00018,places=12)
  self.assertGreater(S['signed_volume'](vs,fs),0)
 def test_forward_free_tip_and_lateral_reach_are_separate_bounded_controls(self):
  self.assertGreater(STYLE['accent_tuft_flare_m'],.002)
  self.assertLess(STYLE['accent_tuft_flare_m'],.018)
  self.assertLess(max(p[0] for p in STYLE['outer_lash_wing_rails']),1.35)
  self.assertLess(STYLE['accent_band_thickness_m'],.0003)
 def test_join_transition_is_continuous_and_mirrored(self):
  last=S['lateral_accent_point'](1,0,0,0)
  for i in range(1,101):
   p=S['lateral_accent_point'](1,i/100,0,0)
   q=S['lateral_accent_point'](-1,i/100,0,0)
   self.assertTrue(all(math.isfinite(v) for v in p))
   self.assertLess(math.dist(p,last),.002)
   self.assertAlmostEqual(p[0],-q[0]);self.assertAlmostEqual(p[1],q[1]);self.assertAlmostEqual(p[2],q[2])
   last=p

 def test_free_upper_tip_moves_forward_and_up_without_moving_attached_root(self):
  key='upper_swept_tuft_outline'; tip=max(STYLE[key],key=lambda p:p[1])
  q=S['upper_accent_point'](1,tip[0],tip[1],0,key)
  root=S['upper_accent_point'](1,tip[0],S['curve'](tip[0],2,STYLE['upper_lash_rails']),0)
  self.assertLess(q[1],root[1]-.006)
  self.assertGreater(q[2],root[2]+.002)
  ratio=(q[2]-root[2])/(root[1]-q[1])
  self.assertGreater(ratio,.15)  # calibrated shallow upturned leaf, not vertical spike
  self.assertLess(ratio,.85)
  wing=STYLE['outer_lash_wing_rails']
  self.assertLess(wing[-1][1],wing[0][1])  #432 deliberately turns free outer wing down

 def test_free_tip_projects_ahead_of_entire_attached_root_curve(self):
  key='upper_swept_tuft_outline'; tip=max(STYLE[key],key=lambda p:p[1])
  for side in [-1,1]:
   q=S['upper_accent_point'](side,tip[0],tip[1],0,key)
   roots=[S['upper_accent_point'](side,t,S['curve'](t,2,STYLE['upper_lash_rails']),0) for t,_ in STYLE[key]]
   self.assertLess(q[1],min(p[1] for p in roots)-.006)

 def test_main_lower_free_margin_faces_upward_from_upper_attached_root(self):
  for side in [-1,1]:
   for t in [-.5,0,.5,.85]:
    lo=S['curve'](t,1,STYLE['upper_lash_rails']);hi=S['curve'](t,2,STYLE['upper_lash_rails'])
    root=S['upper_accent_point'](side,t,hi,0);free=S['upper_accent_point'](side,t,lo,0)
    self.assertLess(free[2],root[2])
    # 363 narrows the outer fork; forward depth scales with strip width.
    # Retain the 2 mm central-band requirement and analytic positive pitch.
    expected_pitch=(hi-lo)*S['H']*STYLE['accent_sheet_slope']
    self.assertGreater(expected_pitch,0)
    self.assertLessEqual(free[1],root[1]-min(.002,expected_pitch)+1e-12)
