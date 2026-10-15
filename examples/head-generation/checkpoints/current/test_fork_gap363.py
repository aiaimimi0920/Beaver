import unittest
from test_spatial_lashes196 import STYLE,S
class ForkGapTests(unittest.TestCase):
 def test_outer_fork_is_narrow_not_open_v(self):
  for t in [.86,.94,.99]:
   gap=S['curve'](t,1,STYLE['upper_lash_rails'])-S['curve'](t,2,STYLE['outer_lash_wing_rails'])
   self.assertGreater(gap,-.002)
   self.assertLess(gap,.005)
 def test_central_arch_has_bounded_height(self):
  p=next(r for r in STYLE['upper_lash_rails'] if r[0]==.2)
  self.assertEqual(p[2],.055)
