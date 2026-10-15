import unittest
from test_style_math import pure_function
f=pure_function('eye_accents.py','final_free_rail_delta',{})[0]
class FinalHingeTests(unittest.TestCase):
 def test_reversed_station_moves_only_free_edge_forward(self):
  self.assertAlmostEqual(f(-.070,-.071,.004,.25,.004),-.002)
 def test_already_forward_station_unchanged(self):
  self.assertEqual(f(-.074,-.071,.004,.25,.004),0)
 def test_coincident_tip_unchanged(self):
  self.assertEqual(f(-.071,-.071,0,.25,.004),0)
 def test_refuses_excessive_displacement(self):
  with self.assertRaises(AssertionError):f(-.06,-.071,.004,.25,.004)
 def test_translation_invariant(self):
  self.assertAlmostEqual(f(-.05,-.051,.004,.25,.004),f(-.07,-.071,.004,.25,.004))

 def test_zero_base_pitch_still_prevents_backward_free_edge(self):
  self.assertAlmostEqual(f(-.070,-.071,.004,0,.004),-.001)
 def test_zero_base_pitch_preserves_forward_edge(self):
  self.assertEqual(f(-.074,-.071,.004,0,.004),0)
