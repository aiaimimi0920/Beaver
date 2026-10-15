import unittest
from test_style_math import pure_function
f=pure_function('eye_accents.py','fitted_crown_relief',{})[0]
class CrownWidthTests(unittest.TestCase):
 def test_collapsed_tip_has_no_relief(self):self.assertEqual(f(0,.236,.0045,.5),0)
 def test_half_width_has_half_relief(self):self.assertAlmostEqual(f(.007*.236,.236,.0045,1),.00225)
 def test_wide_middle_retains_relief(self):self.assertEqual(f(.03*.236,.236,.0045,1),.0045)
 def test_continuous_at_threshold(self):self.assertLess(abs(f(.014*.236-1e-10,.236,.0045,1)-.0045),1e-8)
