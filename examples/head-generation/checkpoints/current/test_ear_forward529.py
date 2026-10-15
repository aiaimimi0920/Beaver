import unittest
from ear_boundary import support_offset
class EarForward529(unittest.TestCase):
 def test_signed_bounded_forward_shift(self):
  for i in range(101):
   v=i/100;d=support_offset(v,1,-.006)
   self.assertTrue(-.006000001<=d<=0)
   if v<=.18 or v>=.34:self.assertAlmostEqual(d,0)
  self.assertAlmostEqual(support_offset(.26,1,-.006),-.006)
