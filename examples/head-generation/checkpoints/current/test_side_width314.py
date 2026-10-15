import unittest
from test_style_math import STYLE,pure_function
curve,_=pure_function('face.py','curve',{})
class SideWidthTests(unittest.TestCase):
 def test_top_widest_monotonic_taper_and_pointed_tip(self):
  p=STYLE['accent_lateral_inner_width_profile']
  self.assertAlmostEqual(p[0][1],.6857142857142856)
  self.assertEqual(p[-1][1],0)
  vals=[curve(i/100,1,p) for i in range(101)]
  self.assertTrue(all(a>b for a,b in zip(vals,vals[1:])))
  self.assertTrue(all(v>=0 for v in vals))
  self.assertLess(vals[75],vals[0]*.36)
