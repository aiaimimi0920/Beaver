import unittest
from ear_boundary import rear_curve_offset
class RearCurve489(unittest.TestCase):
 def test_bounded_support(self):
  for i in range(101):
   v=i/100
   for depth in [-1,0,.1,.18,.20,.23,.3,.4]:
    d=rear_curve_offset(v,depth,.007)
    self.assertGreaterEqual(d,0);self.assertLessEqual(d,.00700000001)
    if v<=.025 or v>=.27 or depth<=.18:self.assertAlmostEqual(d,0,places=12)
  self.assertAlmostEqual(rear_curve_offset(.11,.3,.007),.007)
 def test_smooth_attachment(self):
  e=1e-7
  for v in [.025,.11,.27]:
   self.assertLess(abs(rear_curve_offset(v+e,.3,.007)-rear_curve_offset(v-e,.3,.007))/(2*e),1e-5)
