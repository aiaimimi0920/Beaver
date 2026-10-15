import unittest
from test_style_math import pure_function
class EarCrown498(unittest.TestCase):
 def test_bounded_crown_and_fixed_ends(self):
  fn,_=pure_function('ears.py','return_crown_point',{})
  a=(.10,.08,1.62);b=(.08,.07,1.62)
  for i in range(101):
   t=i/100;p=fn(a,b,t,.002)
   self.assertAlmostEqual(p[0],.09);self.assertAlmostEqual(p[2],1.62)
   self.assertGreaterEqual(p[1],.075);self.assertLessEqual(p[1],.077+1e-12)
   if i in (0,100):self.assertAlmostEqual(p[1],.075)
