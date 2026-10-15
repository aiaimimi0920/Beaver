import unittest,math
from lateral_fit import continuous_guide_moves
class LateralGuideTests(unittest.TestCase):
 def test_preserves_endpoints_and_height(self):
  b=[(.08,-.1,1),(.079,-.099,0.9),(.077,-.099,0.8)]
  d=continuous_guide_moves([0,.5,1],b)
  self.assertEqual(d[0],(0,0,0));self.assertEqual(d[-1],(0,0,0));self.assertEqual(d[1][2],0)
 def test_guide_has_no_lateral_reversal(self):
  f=[i/10 for i in range(11)];b=[(.08-.003*t,-.1+.001*t,1-t) for t in f]
  d=continuous_guide_moves(f,b);x=[a[0]+v[0] for a,v in zip(b,d)]
  self.assertTrue(all(a>b for a,b in zip(x,x[1:])))
 def test_large_displacement_is_rejected(self):
  with self.assertRaises(AssertionError):continuous_guide_moves([0,.5,1],[(0,0,0),(.01,0,0),(0,0,0)])
