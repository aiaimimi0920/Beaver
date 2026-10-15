import unittest
from lateral_fit import wing_depth_shift
class WingSeatingTests(unittest.TestCase):
 def test_root_is_fixed(self):self.assertEqual(wing_depth_shift(-.1,-.097,0),0)
 def test_tip_seats_and_can_move_both_directions(self):
  self.assertAlmostEqual(wing_depth_shift(-.1,-.097,1),.003)
  self.assertAlmostEqual(wing_depth_shift(-.097,-.1,1),-.003)
 def test_large_move_rejected(self):
  with self.assertRaises(AssertionError):wing_depth_shift(-.1,-.09,1)
 def test_blend_is_monotone(self):
  d=[wing_depth_shift(-.1,-.097,f/10) for f in range(11)]
  self.assertTrue(all(a<=b for a,b in zip(d,d[1:])))
