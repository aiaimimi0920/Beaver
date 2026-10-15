import unittest,math
from lateral_fit import companion_curve
class CompanionCurve(unittest.TestCase):
 def test_endpoints_and_finite(self):
  a=(.075,-.075,1.65);b=(.065,-.08,1.63)
  for t in [0,1]:
   p=companion_curve(a,b,1,t);q=a if t==0 else b
   self.assertAlmostEqual(p[0],q[0]-.00008);self.assertAlmostEqual(p[1],q[1]-.0002);self.assertEqual(p[2],q[2])
  self.assertTrue(all(math.isfinite(v) for i in range(101) for v in companion_curve(a,b,1,i/100)))
 def test_mirrored_and_independent(self):
  a=(.075,-.075,1.65);b=(.065,-.08,1.63)
  for i in range(101):
   t=i/100;p=companion_curve(a,b,1,t);q=companion_curve((-a[0],a[1],a[2]),(-b[0],b[1],b[2]),-1,t)
   self.assertAlmostEqual(p[0],-q[0]);self.assertEqual(p[1:],q[1:])
  self.assertGreater(companion_curve(a,b,1,.5)[0],(a[0]+b[0])/2+.001)
