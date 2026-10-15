import unittest
from test_spatial_lashes196 import S,STYLE
class LowerArch378(unittest.TestCase):
 def test_inner_arch_and_continuous_outer_rise(self):
  c=lambda t:S['curve'](t,1,STYLE['upper_lash_rails'])
  self.assertGreater(c(-.144),c(-.704)+.003)
  self.assertGreater(c(.65),c(-.144))
  values=[c(-.704+i*(.65+.704)/200) for i in range(201)]
  self.assertTrue(all(b>=a-1e-10 for a,b in zip(values,values[1:])))
  self.assertLess(c(.65),c(.86))
 def test_shared_root(self):
  self.assertAlmostEqual(S['curve'](.65,1,STYLE['upper_lash_rails']),STYLE['outer_lash_wing_rails'][0][1])
 def test_positive_section_width(self):
  for i in range(1,225):
   lo,hi=STYLE["upper_lash_rails"][0][0],STYLE["upper_lash_rails"][-1][0]
   t=lo+(hi-lo)*i/225
   self.assertGreater(S['curve'](t,2,STYLE['upper_lash_rails'])-S['curve'](t,1,STYLE['upper_lash_rails']),0)
