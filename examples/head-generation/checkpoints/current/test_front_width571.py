import unittest
from ear_width import target_width,PROFILE
class FrontEnvelope(unittest.TestCase):
 def test_knots_and_continuity(self):
  for h,w in PROFILE:self.assertAlmostEqual(target_width(h),w)
  values=[target_width(-.4+.15*i/1000) for i in range(1000)]
  self.assertTrue(all(a<=b for a,b in zip(values,values[1:])))
 def test_no_effect_outside_scope(self):
  self.assertIsNone(target_width(-.401));self.assertIsNone(target_width(-.249))
