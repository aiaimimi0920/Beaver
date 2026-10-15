import unittest
from eye_rim_refine import outer_eye_delta
class LocalOuterEye(unittest.TestCase):
 def test_scope_symmetry_and_bound(self):
  for x in [0,.02,.055,.056,.065,.078,.085,.09,.1]:
   for z in [1.58,1.60,1.61,1.63,1.65,1.665,1.7]:
    d=outer_eye_delta(x,-.08,z);self.assertEqual(d,outer_eye_delta(-x,-.08,z));self.assertLessEqual(abs(d),.0025)
    if x<=.052 or x>=.09 or z<=1.6 or z>=1.665:self.assertEqual(d,0)
    self.assertEqual(outer_eye_delta(x,0,z),0)
 def test_outer_middle_recedes(self):
  self.assertAlmostEqual(outer_eye_delta(.075,-.08,1.635),.0025)
