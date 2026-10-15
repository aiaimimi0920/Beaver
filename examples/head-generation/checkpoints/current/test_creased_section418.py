import unittest
from main_curve import curved_section
class CreasedSection418(unittest.TestCase):
 def test_creased_panels_keep_boundaries_crest_and_thin_return(self):
  lo=(0,0,0);hi=(0,0,.01);crest=(0,-.003,.005)
  r=curved_section(lo,crest,hi,.00018,"creased")
  self.assertEqual(r[1],lo);self.assertEqual(r[3],crest);self.assertEqual(r[5],hi)
  self.assertAlmostEqual(r[2][1],-.0015);self.assertAlmostEqual(r[4][1],-.0015)
  for a,b in [(1,0),(2,9),(3,8),(4,7),(5,6)]:self.assertAlmostEqual(r[b][1]-r[a][1],.00018)
 def test_unknown_profile_rejected(self):
  with self.assertRaises(AssertionError):curved_section((0,0,0),(0,-.003,.005),(0,0,.01),.00018,"invalid")
