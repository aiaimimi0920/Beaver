import unittest
from ear_boundary import constrain_lower_envelope
class Envelope533(unittest.TestCase):
 def test_fixed_height_and_unselected_points(self):
  p=[(0,0,0),(0,0,1),(.6,-.3,.7),(.6,.3,.2),(-.6,.3,.2)]
  q=constrain_lower_envelope(p,p,{3,4})
  self.assertEqual(q[:3],p[:3]);self.assertEqual(q[3][2],p[3][2])
  self.assertAlmostEqual(q[3][0],-q[4][0]);self.assertAlmostEqual(q[3][1],q[4][1])
  self.assertLessEqual(sum((q[3][k]-p[3][k])**2 for k in range(3))**.5,.003000001)
