import unittest
from test_style_math import pure_function
f=pure_function('eye_accents.py','fitted_crown_depth',{})[0]
class FinalCrownTests(unittest.TestCase):
 def test_relief_is_relative_to_final_boundaries(self):
  self.assertAlmostEqual(f(-.07,-.071,.4,.0015),-.0719)
 def test_translation_invariance(self):
  self.assertAlmostEqual(f(-.05,-.051,.4,.0015)-f(-.07,-.071,.4,.0015),.02)
 def test_zero_relief_is_on_boundary_chord(self):
  self.assertAlmostEqual(f(-.07,-.071,.5,0),-.0705)
 def test_positive_relief_is_forward(self):
  self.assertLess(f(-.07,-.071,.5,.001),f(-.07,-.071,.5,0))
