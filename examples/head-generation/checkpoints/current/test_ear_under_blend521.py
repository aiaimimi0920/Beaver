import unittest
from ear_boundary import under_ear_lateral_offset

class EarUnderBlend521(unittest.TestCase):
    def test_extended_upper_falloff_is_monotonic(self):
        f=lambda v:under_ear_lateral_offset(v,.23,.08,.003)
        ys=[f(.22+.14*i/100) for i in range(101)]
        self.assertEqual(ys,sorted(ys,reverse=True))
        self.assertAlmostEqual(f(.36),0)
        self.assertGreater(f(.29),.001)
    def test_end_slopes_are_continuous(self):
        f=lambda v:under_ear_lateral_offset(v,.23,.08,.003)
        e=1e-7
        for v in (.13,.22,.36):
            self.assertLess(abs(f(v+e)-f(v-e))/(2*e),1e-5)
