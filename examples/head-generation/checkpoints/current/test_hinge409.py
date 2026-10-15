import unittest
from test_style_math import pure_function
f=pure_function("eye_accents.py","final_free_rail_delta",{})[0]
class Hinge409(unittest.TestCase):
 def test_recessed_guide_reduces_artificial_projection(self):
  old=f(-.067,-.070,.006,0,.004)
  new=f(-.067,-.070,.006,-.3,.004)
  self.assertAlmostEqual(old,-.003)
  self.assertAlmostEqual(new,-.0012)
  self.assertGreater(new,old)
 def test_does_not_push_already_clear_free_edge_backward(self):
  self.assertEqual(f(-.075,-.070,.006,-.3,.004),0)
 def test_same_displacement_guard(self):
  with self.assertRaises(AssertionError):f(-.060,-.070,.006,-.3,.004)
