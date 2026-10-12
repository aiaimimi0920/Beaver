import unittest
from ear_boundary import under_ear_lateral_offset
class EarUnderWidth(unittest.TestCase):
 def test_bounded_local_support(self):
  f=under_ear_lateral_offset
  self.assertAlmostEqual(f(.22,.23,.06,.003),.003)
  for i in range(101):
   v=i/100
   for d in (.1,.18,.2,.23,.4):
    for x in (0,.025,.04,.06,.09):
     y=f(v,d,x,.003);self.assertGreaterEqual(y,0);self.assertLessEqual(y,.0030000001)
     if v<=.13 or v>=.29 or d<=.18 or x<=.025:self.assertEqual(y,0)
