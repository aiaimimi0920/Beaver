import unittest
from lateral_fit import continuous_guide_moves
class OutwardBow384(unittest.TestCase):
 def test_bounded_mirrored_bow_and_fixed_endpoints(self):
  fs=[i/20 for i in range(21)]
  for side in (-1,1):
   bases=[(side*(.08-.003*f**1.8),-.1+.001*f,1-f) for f in fs]
   shifts=continuous_guide_moves(fs,bases)
   self.assertEqual(shifts[0],(0,0,0));self.assertEqual(shifts[-1],(0,0,0))
   for f,d in zip(fs,shifts):
    self.assertAlmostEqual(side*d[0],.003*(f**1.8-f**2.0),places=12)
    self.assertAlmostEqual(d[1],0,places=12);self.assertEqual(d[2],0)
