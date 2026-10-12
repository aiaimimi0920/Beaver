import unittest
from test_style_math import pure_function
class OpeningArc503(unittest.TestCase):
 def test_smooth_bounded_lower_arc(self):
  fn,_=pure_function('ear_boundary.py','opening_lateral_offset',{})
  self.assertEqual(fn(.04,.006),0);self.assertEqual(fn(.25,.006),0)
  self.assertAlmostEqual(fn(.11,.006),.006)
  for i in range(1001):
   v=i/1000;d=fn(v,.006);self.assertGreaterEqual(d,0);self.assertLessEqual(d,.006)
   if v<=.04 or v>=.25:self.assertEqual(d,0)
