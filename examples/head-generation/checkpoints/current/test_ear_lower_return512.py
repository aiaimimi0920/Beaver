import unittest
from ear_return import lower_return_weight
class LowerReturnWidth(unittest.TestCase):
 def test_compact_lower_return(self):
  self.assertEqual(lower_return_weight(.01),0)
  self.assertEqual(lower_return_weight(.115),0)
  self.assertAlmostEqual(lower_return_weight(.055),1)
  for i in range(1001):
   v=i/1000;w=lower_return_weight(v)
   self.assertGreaterEqual(w,0);self.assertLessEqual(w,1)
   if v<=.01 or v>=.115:self.assertEqual(w,0)
