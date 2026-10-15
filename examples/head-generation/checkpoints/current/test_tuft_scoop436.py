import unittest
from test_style_math import pure_function
f=pure_function("eye_accents.py","tuft_rear_scoop",{})[0]
class Scoop(unittest.TestCase):
 def test_fixed_root_and_tip(self):
  self.assertEqual(f(0,.004),0);self.assertEqual(f(1,.004),0)
 def test_bounded_rear_scoop(self):
  self.assertAlmostEqual(f(.225,.004),.004)
  for i in range(101):self.assertGreaterEqual(f(i/100,.004),0);self.assertLessEqual(f(i/100,.004),.004+1e-12)

 def test_free_half_is_unmodified(self):
  for p in [.45,.5,.7,.9,1]:self.assertEqual(f(p,.004),0)
