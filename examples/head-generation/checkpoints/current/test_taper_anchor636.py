import unittest
from lateral_fit import taper_anchor_shift,sample_profile
class TaperAnchorTests(unittest.TestCase):
 def test_inner_edge_is_preserved_and_outer_edge_moves_inward(self):
  d=taper_anchor_shift((.08,0,1),(.082,0,1),-.01,0,-.015)
  self.assertAlmostEqual(d[0],-.001);self.assertEqual(d[2],0)
 def test_unchanged_root_has_zero_shift(self):
  self.assertEqual(taper_anchor_shift((.08,0,1),(.082,0,1),-.01,0,-.01),(0,0,0))
 def test_rejects_excessive_shift(self):
  with self.assertRaises(AssertionError):taper_anchor_shift((.08,0,1),(.082,0,1),-.01,0,-.04)
