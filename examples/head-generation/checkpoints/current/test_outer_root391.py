import unittest
from test_spatial_lashes196 import S,STYLE
class OuterRoot391(unittest.TestCase):
 def test_physical_outer_lower_edge_has_no_dip(self):
  p=lambda t:S['upper_accent_point'](1,t,S['curve'](t,1,STYLE['upper_lash_rails']),0.0)
  self.assertLess(p(.55)[2],p(.65)[2]);self.assertLess(p(.65)[2],p(.86)[2])
  self.assertEqual(STYLE['outer_lash_wing_rails'][0],[.65,.0227,.0227])
