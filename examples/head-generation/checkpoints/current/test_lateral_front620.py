import unittest,math
from lateral_fit import front_readable_frames
class FrontalFrames620(unittest.TestCase):
 def test_endpoints_and_bounded_orientation(self):
  s=[i/100 for i in range(101)];a=[-1.2]*101;b=front_readable_frames(s,a,.35)
  self.assertEqual(b[0],a[0]);self.assertEqual(b[-1],a[-1]);self.assertAlmostEqual(b[50],-.78)
  self.assertTrue(all(abs(v)<=1.2 and v<0 for v in b));self.assertLess(max(abs(b[i+1]-b[i]) for i in range(100)),.02)
 def test_invalid_bias_rejected(self):
  with self.assertRaises(AssertionError):front_readable_frames([0,.5,1],[1,1,1],.7)
