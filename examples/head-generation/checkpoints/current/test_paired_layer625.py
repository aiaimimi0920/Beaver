import unittest
from lateral_fit import paired_layer_delta
class Pairing(unittest.TestCase):
 def test_target_gap_and_height(self):
  e=(.04,-.08,1.63);t=(.041,-.081,1.63);d=paired_layer_delta(e,t,1)
  self.assertAlmostEqual(e[0]+d[0],t[0]-.00008);self.assertAlmostEqual(e[1]+d[1],t[1]-.0002);self.assertEqual(d[2],0)
 def test_mirror(self):
  a=paired_layer_delta((.04,-.08,1.63),(.041,-.081,1.63),1);b=paired_layer_delta((-.04,-.08,1.63),(-.041,-.081,1.63),-1)
  self.assertAlmostEqual(a[0],-b[0]);self.assertEqual(a[1:],b[1:])
