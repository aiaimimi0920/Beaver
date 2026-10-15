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
    def test_secondary_blade_meets_parent_guide_except_at_short_tip(self):
        # Owner191: keep the long spans open; the short outward tip joins
        # the companion locally and projects beyond it as an intentional tuft.
        self.assertLess(STYLE['accent_lateral_branch_separation_H'],.01)
        for t in [.25,.78]:
            edge=S['curve'](t,1,STYLE['lateral_accent_profile'])
            start=STYLE['accent_lateral_branch_separation_H']*math.sin(math.pi*t)**.85
            self.assertAlmostEqual((start-edge)*S['H'],0.0,places=8)
            self.assertLess((start-edge)*S['H'],.002)
        t=.65
        edge=S['curve'](t,1,STYLE['lateral_accent_profile'])
        start=STYLE['accent_lateral_branch_separation_H']*math.sin(math.pi*t)**.85
        self.assertGreater((edge-start)*S['H'],.001)
        self.assertLess((edge-start)*S['H'],.0052)
    def test_small_lower_tuft_is_bounded_to_outer_lid(self):
        self.assertGreater(STYLE['accent_lower_tip_t'],.3)
        self.assertLess(STYLE['accent_lower_tip_t'],.8)
        self.assertLess(STYLE['accent_lower_tip_drop_H']*S['H'],.003)
        self.assertLess(STYLE['accent_lower_tip_width_t'],.1)
