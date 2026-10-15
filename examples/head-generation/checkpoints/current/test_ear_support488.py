import unittest
from ear_boundary import support_offset
class EarSupport488(unittest.TestCase):
 def test_compact_support(self):
  for i in range(101):
   v=i/100
   for q in [0,.5,.85,.9,.92,.95,1,1.5]:
    d=support_offset(v,q,.003)
    self.assertGreaterEqual(d,0);self.assertLessEqual(d,.00300000001)
    if v<=.18 or v>=.34 or q<=.9:self.assertAlmostEqual(d,0,places=12)
  self.assertAlmostEqual(support_offset(.26,1,.003),.003)
