import unittest
from lateral_fit import seating_delta
class LateralSeatingTests(unittest.TestCase):
 def test_parent_root_and_height_are_preserved(self):
  self.assertEqual(seating_delta((0,0,0),(.001,.001,.002),(0,-1,0),0,.00035),(0,0,0))
  self.assertEqual(seating_delta((0,0,0),(.001,.001,.002),(0,-1,0),.5,.00035)[2],0)
 def test_seating_reaches_clearance_offset_support(self):
  d=seating_delta((0,0,0),(.001,.001,.002),(0,-1,0),.5,.00035)
  self.assertAlmostEqual(d[0],.001);self.assertAlmostEqual(d[1],.00065)
 def test_excessive_seating_is_rejected(self):
  with self.assertRaises(AssertionError):seating_delta((0,0,0),(.01,0,0),(0,-1,0),1,.00035)
