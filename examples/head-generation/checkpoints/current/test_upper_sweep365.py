import unittest
from test_spatial_lashes196 import S,STYLE
class UpperSweep365(unittest.TestCase):
 def test_upper_branch_has_no_down_up_notch(self):
  values=[S['curve'](.2+i*.009,2,STYLE['upper_lash_rails']) for i in range(101)]
  for a,b in zip(values,values[1:]):self.assertLessEqual(b,a+1e-12)
 def test_fixed_lower_branch_root(self):
  self.assertAlmostEqual(S['curve'](.65,1,STYLE['upper_lash_rails']),STYLE['outer_lash_wing_rails'][0][1])
