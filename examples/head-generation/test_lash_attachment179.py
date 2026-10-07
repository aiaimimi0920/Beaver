import math
import unittest
from test_style_math import pure_function, STYLE

C=STYLE['calibration']
S={'STYLE':STYLE,'C':C,'H':C['height_m'],'Y0':C['origin_y_m'],'Z0':C['chin_depth_m']}
for n in ['curve','width','nose_profile_factor','nose','lip_shoulder_support','lip_detail_depth','base_depth','rim_depth','depth','coord','eye_contour','lash_depth','upper_ink_depth']:
    S[n]=pure_function('face.py',n,S)[0]
for n in ['upper_lid_support','upper_attached_depth','lateral_position','outline_sections']:
    S[n]=pure_function('eye_accents.py',n,S)[0]

class LashAttachmentTests(unittest.TestCase):
    def test_actual_upper_support_clearance_bounded_and_symmetric(self):
        thickness=STYLE['accent_band_thickness_m']
        for i in range(37):
            t=-.9+i*.05
            x,y=S['eye_contour'](1,t,True)
            support=S['upper_lid_support'](x,y)
            attached=S['upper_attached_depth'](x,y,thickness)
            self.assertAlmostEqual(attached-thickness-support,STYLE['accent_surface_clearance_m'])
            self.assertLess(attached-support,.0008)
            self.assertAlmostEqual(attached,S['upper_attached_depth'](-x,y,thickness))
    def test_former_planar_depth_cannot_meet_attachment_contract(self):
        x,y=S['eye_contour'](1,0,True)
        self.assertGreater(S['upper_ink_depth'](x,y)-S['upper_lid_support'](x,y),.007)
    def test_secondary_blade_leaves_open_space_at_midsection(self):
        # Main outer silhouette ends before the thin companion begins.
        t=.55
        old_peak=max(p[1] for p in STYLE['lateral_accent_profile'])
        branch_start=STYLE['accent_lateral_branch_separation_H']*math.sin(math.pi*t)**.85
        self.assertGreater((branch_start-old_peak)*S['H'],.001)
    def test_small_lower_tuft_is_bounded_to_outer_lid(self):
        self.assertGreater(STYLE['accent_lower_tip_t'],.3)
        self.assertLess(STYLE['accent_lower_tip_t'],.8)
        self.assertLess(STYLE['accent_lower_tip_drop_H']*S['H'],.003)
        self.assertLess(STYLE['accent_lower_tip_width_t'],.1)
