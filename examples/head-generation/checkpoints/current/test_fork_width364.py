import unittest
from test_spatial_lashes196 import S,STYLE
class ForkWidth364(unittest.TestCase):
 def test_outer_branch_has_readable_width(self):
  for t,w in [(.86,.015),(.99,.009)]:
   self.assertAlmostEqual(S['curve'](t,2,STYLE['upper_lash_rails'])-S['curve'](t,1,STYLE['upper_lash_rails']),w)
 def test_wing_root_matches_shared_attachment(self):
  self.assertAlmostEqual(S['curve'](.65,1,STYLE['upper_lash_rails']),STYLE['outer_lash_wing_rails'][0][1])
