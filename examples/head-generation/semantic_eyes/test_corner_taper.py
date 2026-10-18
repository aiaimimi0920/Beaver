"""Opt-in actual aperture taper; legacy power family remains unchanged."""
import json
import unittest
from bundle import RECIPE
from parameters import adapt_style, validate, upper_change
from test_lower_contour import contour
from blink_runtime import closure_delta

class CornerTaperTests(unittest.TestCase):
    def setUp(self):
        self.original=json.loads((RECIPE/'face_style.json').read_text())
    def test_default_identity_and_bounded_dependency(self):
        self.assertEqual(adapt_style(self.original,{'aperture_corner_taper':0}),self.original)
        for v in (-.01,1.01,float('nan'),True):
            with self.assertRaises(ValueError):validate({'aperture_corner_taper':v,'lash_contour_follow':1})
        with self.assertRaises(ValueError):validate({'aperture_corner_taper':1})
    def test_real_contour_finite_angle_and_same_center_height(self):
        p=validate({'aperture_corner_taper':1,'lash_contour_follow':1})
        c=adapt_style(self.original,p)['calibration'];old=self.original['calibration']
        self.assertEqual(c['eye_upper_power'],1);self.assertEqual(c['eye_lower_power'],1)
        style=adapt_style(self.original,p)
        self.assertEqual(style['accent_lateral_guide_inward_power'],1)
        self.assertEqual(style['accent_lateral_guide_bow_fraction'],1)
        f=contour(c);base=contour(old)
        for side in (-1,1):
            for upper in (True,False):
                self.assertEqual(f(side,0,upper),base(side,0,upper))
                for end in (-1,1):
                    x,y=f(side,end,upper)
                    slopes=[]
                    for eps in (1e-3,1e-4,1e-5):
                        xx,yy=f(side,end*(1-eps),upper);slopes.append((yy-y)/(xx-x))
                    self.assertLess(abs(slopes[-1]-slopes[-2]),.002)
            for t in (-.99,-.6,0,.6,.99):
                x,y=f(side,t,True)
                _,before=base(side,t,True)
                self.assertAlmostEqual(y-before,old['height_m']*upper_change(t,old,p))
                for up in (True,False):
                    xx,yy=f(side,t,up)
                    target=c['origin_y_m']+c['height_m']*(c['eye_corner_y']+c['eye_corner_slope']*t)-.0015*(1-t*t)
                    self.assertAlmostEqual(yy+closure_delta(xx,yy,'upper' if up else 'lower',c),target)

    def test_803_combination_actual_warp_and_outer_inverse(self):
        import ast
        from pathlib import Path
        from test_upper_warp import UpperWarpTests
        from source_upper_warp import adapt_upper_sources
        from contour_warp import upper_shape
        eyes=json.loads((Path(__file__).parents[1]/'eye_presets/almond-balanced-796.json').read_text())['eye_parameters']
        eyes.update(aperture_corner_taper=1,lateral_aperture_follow=1,lateral_blink_follow=.70)
        p=validate(eyes);c=adapt_style(self.original,p)['calibration']
        f=UpperWarpTests().contour(c,True)
        sources={n:(RECIPE/n).read_text() for n in ('face.py','eye_socket.py','lateral_fit.py')}
        src=adapt_upper_sources(sources,True)['lateral_fit.py']
        node=next(n for n in ast.parse(src).body if isinstance(n,ast.FunctionDef) and n.name=='eye_rim_x_at_height')
        env={'upper_shape':upper_shape};exec(compile(ast.Module(body=[node],type_ignores=[]),'<inverse>','exec'),env)
        for side in (-1,1):
            for i in range(1,100):
                t=-1+i/50
                for upper in (True,False):
                    x,y=f(side,t,upper)
                    seam=c['origin_y_m']+c['height_m']*(c['eye_corner_y']+c['eye_corner_slope']*t)-.0015*(1-t*t)
                    self.assertAlmostEqual(y+closure_delta(x,y,'upper' if upper else 'lower',c),seam)
                old=self.original['calibration'];expected=(f(side,t,True)[1]-c['origin_y_m'])/c['height_m']-c['eye_corner_y']-old['eye_upper_arch']*upper_shape(t,old)
                self.assertAlmostEqual(upper_change(t,old,p),expected)
        for t in (.4,.5,.7,.9,.999):
            x,y=f(1,t,True);self.assertAlmostEqual(env['eye_rim_x_at_height'](y,c),x,places=12)
