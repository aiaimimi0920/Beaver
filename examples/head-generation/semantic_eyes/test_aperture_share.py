"""Explicit physical aperture allocation instead of unequal base multipliers."""
import json
import unittest
from bundle import RECIPE
from parameters import adapt_style, validate, upper_change
from contour_warp import upper_shape
from blink_runtime import closure_delta

class ApertureShareTests(unittest.TestCase):
    def test_disabled_preserves_style_and_active_bounds(self):
        original=json.loads((RECIPE/'face_style.json').read_text())
        self.assertEqual(adapt_style(original,{'aperture_upper_share':0}),original)
        for v in (.01,.199,.801,True,float('nan')):
            with self.assertRaises(ValueError):validate({'aperture_upper_share':v,'lash_contour_follow':1})
        with self.assertRaises(ValueError):validate({'aperture_upper_share':.7})
    def test_height_sum_iris_shift_and_real_contour_blink(self):
        from test_upper_warp import UpperWarpTests
        original=json.loads((RECIPE/'face_style.json').read_text());old=original['calibration']
        for share in (.2,.5,.72,.8):
            from pathlib import Path
            request=json.loads((Path(__file__).parents[1]/'eye_presets/almond-taper-experiment-803.json').read_text())['eye_parameters']
            request['aperture_upper_share']=share
            request['lateral_guide_inward_power']=2
            request['upper_corner_softness']=.65
            request['upper_horizontal_roundness']=.95
            request['upper_asymmetry']=.08
            p=validate(request)
            c=adapt_style(original,p)['calibration'];total=old['eye_upper_arch']+old['eye_lower_arch']*.68
            self.assertAlmostEqual(c['eye_upper_arch']+c['eye_lower_arch'],total)
            self.assertAlmostEqual(c['eye_upper_arch']/total,share)
            self.assertGreater(c['eye_lower_arch'],0)
            self.assertAlmostEqual(c['iris_center_y']-old['iris_center_y'],total*share-old['eye_upper_arch'])
            f=UpperWarpTests().contour(c,True)
            for i in range(1,100):
                t=-1+i/50
                for upper in (True,False):
                    x,y=f(1,t,upper);seam=c['origin_y_m']+c['height_m']*c['eye_corner_y']-.0015*(1-t*t)
                    self.assertAlmostEqual(y+closure_delta(x,y,'upper' if upper else 'lower',c),seam)
                expected=(f(1,t,True)[1]-c['origin_y_m'])/c['height_m']-c['eye_corner_y']-old['eye_upper_arch']*upper_shape(t,old)
                self.assertAlmostEqual(upper_change(t,old,p),expected)

    def test_independent_guide_power_keeps_default_and_bounds(self):
        original=json.loads((RECIPE/'face_style.json').read_text())
        self.assertEqual(adapt_style(original,{'lateral_guide_inward_power':0}),original)
        for power in (1,2,3,4):
            self.assertEqual(adapt_style(original,{'lateral_guide_inward_power':power})['accent_lateral_guide_inward_power'],power)
        for power in (.01,.99,4.01,True):
            with self.assertRaises(ValueError):validate({'lateral_guide_inward_power':power})

    def test_upper_softness_only_changes_upper_profile(self):
        original=json.loads((RECIPE/'face_style.json').read_text())
        self.assertEqual(adapt_style(original,{'upper_corner_softness':0}),original)
        plain=adapt_style(original,{'aperture_corner_taper':1,'lash_contour_follow':1})['calibration']
        soft=adapt_style(original,{'aperture_corner_taper':1,'lash_contour_follow':1,'upper_corner_softness':1})['calibration']
        self.assertEqual(soft['eye_lower_power'],plain['eye_lower_power'])
        self.assertAlmostEqual(soft['eye_upper_power'],.7)
        for v in (-.01,1.01,True,float('nan')):
            with self.assertRaises(ValueError):validate({'upper_corner_softness':v})
