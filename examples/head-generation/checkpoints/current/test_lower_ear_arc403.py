import json,unittest,math
from pathlib import Path
from test_style_math import pure_function
class LowerEarArcTests(unittest.TestCase):
    def test_lower_arc_rises_without_changing_upper_half(self):
        c=json.loads(Path(__file__).with_name('face_style.json').read_text())['calibration'];h=.236
        f,_=pure_function('ears.py','ear_arc_height',{'C':c,'H':h})
        low,high=1.60,1.66
        def old(t):return low+(high-low)*(1-math.cos(math.pi*t))/2+h*c['ear_upper_arc_bias']*t*math.sin(math.pi*t)
        self.assertEqual(f(0,low,high),low)
        self.assertAlmostEqual(f(1,low,high),high)
        for t in [.5,.6,.8,1]:self.assertAlmostEqual(f(t,low,high),old(t),places=12)
        self.assertGreater(f(.08,low,high),old(.08))
        values=[f(i/1000,low,high) for i in range(501)]
        self.assertTrue(all(b>a for a,b in zip(values,values[1:])))
        self.assertLess(max(f(i/1000,low,high)-old(i/1000) for i in range(501)),.003)

    def test_enabled_lower_tangent_matches_declared_rise(self):
        c=json.loads(Path(__file__).with_name('face_style.json').read_text())['calibration']
        f,_=pure_function('ears.py','ear_arc_height',{'C':c,'H':1})
        lo,hi=c['ear_root_min_height'],c['ear_root_max_height']
        slope=(f(.0001,lo,hi)-f(0,lo,hi))/.0001
        self.assertAlmostEqual(slope,(hi-lo)*c['ear_lower_height_linear_blend'],delta=.0001)
