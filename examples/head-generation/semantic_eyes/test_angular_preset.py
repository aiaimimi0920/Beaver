"""Optional upper-shoulder warp, independent of the accepted zero mode."""
import ast
import math
import json
import sys
import unittest
from pathlib import Path
from bundle import RECIPE
from parameters import adapt_style, validate
from contour_warp import eye_warp, eye_warp_derivative, upper_shape, upper_shape_derivative
from blink_runtime import closure_delta
from source_angular_samples import adapt_angular_samples


class AngularPresetTests(unittest.TestCase):
    def setUp(self):
        self.original=json.loads((RECIPE/'face_style.json').read_text())

    def test_angular_edit_does_not_touch_nasal_curve_copy(self):
        face=(RECIPE/'face.py').read_text()
        tree=ast.parse(face)
        curve=next(n for n in tree.body if isinstance(n,ast.FunctionDef) and n.name=='curve')
        nasal=ast.get_source_segment(face,curve).replace('def curve(', 'def nasal_projection_curve(', 1)
        source=face+'\n\n'+nasal+'\n'
        result=adapt_angular_samples({'face.py':source, 'eye_socket.py':(RECIPE/'eye_socket.py').read_text()},True)
        actual=ast.parse(result['face.py'])
        retained=next(n for n in actual.body if isinstance(n,ast.FunctionDef) and n.name=='nasal_projection_curve')
        self.assertEqual(ast.dump(retained,include_attributes=False),ast.dump(ast.parse(nasal).body[0],include_attributes=False))
        self.assertIn('if points is globals().get("STYLE",{}).get("upper_lash_rails"):',result['face.py'])

    def test_identity_bounds_and_level_requirement(self):
        self.assertEqual(adapt_style(self.original,{'upper_angularity':0}),self.original)
        for value in (-.01,1.01,True,float('nan')):
            with self.assertRaises(ValueError):validate({'upper_angularity':value})
        with self.assertRaises(ValueError):adapt_style(self.original,{'upper_angularity':1})

    def test_monotone_shoulder_and_matching_blink(self):
        for g in (0,.25,.65,1):
            for a in (-.25,0,.25):
                c=adapt_style(self.original,dict(upper_angularity=g,upper_asymmetry=a,lash_contour_follow=1,
                    corner_slope_offset=-.014))['calibration']
                samples=[eye_warp(-1+i/1000,c) for i in range(2001)]
                self.assertTrue(all(x<y for x,y in zip(samples,samples[1:])))
                for t in (-.8,-.551,-.549,-.2,.2,.549,.551,.8):
                    numeric=(upper_shape(t+1e-7,c)-upper_shape(t-1e-7,c))/2e-7
                    self.assertAlmostEqual(numeric,upper_shape_derivative(t,c),places=5)
                    x=c['height_m']*(c['eye_center_x']+c['eye_half_width']*t)
                    center=c['origin_y_m']+c['height_m']*c['eye_corner_y']
                    upper=center+c['height_m']*c['eye_upper_arch']*upper_shape(t,c)
                    self.assertAlmostEqual(upper+closure_delta(x,upper,'upper',c),
                        center-.0015*(1-t*t))
        c=adapt_style(self.original,dict(upper_angularity=1,corner_slope_offset=-.014,lash_contour_follow=1))['calibration']
        self.assertGreater(eye_warp_derivative(.550001,c)/eye_warp_derivative(.549999,c),2.8)

    def test_interval_contains_composite_at_crossings(self):
        sys.path.insert(0,str(Path(__file__).parents[1]/'facial_surface'))
        try:from interval_bounds import I,upper_coordinate_interval
        finally:sys.path.pop(0)
        for g in (.25,1):
            for a in (-.25,0,.25):
                c={'eye_upper_asymmetry':a,'eye_upper_angularity':g}
                for lo,hi in [(-1.1,-.9),(-.56,-.54),(-.01,.01),(.54,.56),(.9,1.1),(-1,1)]:
                    bound=upper_coordinate_interval(I(lo,hi),c)
                    for i in range(101):
                        value=eye_warp(lo+(hi-lo)*i/100,c)
                        self.assertLessEqual(bound.lo,value)
                        self.assertGreaterEqual(bound.hi,value)

    def test_explicit_shoulder_sampling_only_when_enabled(self):
        sources={n:(RECIPE/n).read_text() for n in ('face.py','eye_socket.py')}
        self.assertIs(adapt_angular_samples(sources,False),sources)
        changed=adapt_angular_samples(sources,True)
        station_assignment = next(
            node for node in ast.walk(ast.parse(changed['face.py']))
            if isinstance(node, ast.Assign)
            and any(isinstance(target, ast.Name) and target.id == 'stations' for target in node.targets)
        )
        stations = ast.literal_eval(station_assignment.value)
        self.assertEqual(len(stations), 35)
        self.assertTrue({-.55, .55}.issubset(stations))
        self.assertTrue({-1 + 2*i/32 for i in range(33)}.issubset(stations))
        self.assertIn('math.acos(shoulder)',changed['eye_socket.py'])
        style=adapt_style(self.original,dict(upper_angularity=1,corner_slope_offset=-.014,lash_contour_follow=1))
        self.assertTrue({-.55,.55}.issubset({row[0] for row in style['upper_lash_rails']}))

    def test_ink_one_sided_slopes_and_unrelated_curves(self):
        style=adapt_style(self.original,dict(upper_angularity=1,corner_slope_offset=-.014,lash_contour_follow=1))
        sources={n:(RECIPE/n).read_text() for n in ('face.py','eye_socket.py')}
        modified=adapt_angular_samples(sources,True)
        def load(source):
            node=next(n for n in ast.parse(source).body if isinstance(n,ast.FunctionDef) and n.name=='curve')
            scope={'STYLE':style}
            exec(compile(ast.Module(body=[node],type_ignores=[]),'<ink curve>','exec'),scope)
            return scope['curve']
        before,after=load(sources['face.py']),load(modified['face.py'])
        rails=style['upper_lash_rails'];h=1e-7;t=.55
        left=(after(t,1,rails)-after(t-h,1,rails))/h
        right=(after(t+h,1,rails)-after(t,1,rails))/h
        self.assertGreater(abs(left-right),.005)
        independent=[row[:] for row in rails]
        for t in (-.7,-.55,0,.55,.7):
            self.assertEqual(before(t,1,independent),after(t,1,independent))
        self.assertIn('angle = angles[k]',modified['eye_socket.py'])

    def test_interval_exact_shoulders_and_neighbors(self):
        sys.path.insert(0,str(Path(__file__).parents[1]/'facial_surface'))
        try:from interval_bounds import I,upper_coordinate_interval
        finally:sys.path.pop(0)
        for a in (-.25,0,.25):
            c={'eye_upper_asymmetry':a,'eye_upper_angularity':1}
            for t in (-.55,.55):
                for x in (math.nextafter(t,-math.inf),t,math.nextafter(t,math.inf)):
                    bound=upper_coordinate_interval(I(x,x),c);value=eye_warp(x,c)
                    self.assertLessEqual(bound.lo,value);self.assertGreaterEqual(bound.hi,value)


if __name__=='__main__':unittest.main()
