import unittest
from test_spatial_lashes196 import S,STYLE
class UpperArch379(unittest.TestCase):
 def test_soft_middle_arch(self):
  c=lambda t:S['curve'](t,2,STYLE['upper_lash_rails'])
  self.assertGreater(c(.2),c(-.55)+.01)
  self.assertGreater(c(.2),c(.86)+.005)
 def test_outer_tip_unchanged(self):
  self.assertEqual(STYLE['upper_lash_rails'][-1],[1.1,.048,.048])
