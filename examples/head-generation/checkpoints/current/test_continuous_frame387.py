import unittest,math
from lateral_fit import continuous_section_angles,rotate_section
class ContinuousFrame387(unittest.TestCase):
 def test_endpoints_monotonic_and_no_overshoot(self):
  fs=[i/100 for i in range(101)]
  a=continuous_section_angles(fs,.735,.306,.88)
  self.assertEqual(a[0],.735);self.assertAlmostEqual(a[88],.306)
  self.assertTrue(all(a[i]>=a[i+1]-1e-12 for i in range(100)))
  self.assertTrue(all(.306-1e-12<=x<=.735+1e-12 for x in a))
 def test_mirror_and_rigid_section(self):
  fs=[0,.1,.3,.6,.88,1]
  a=continuous_section_angles(fs,.735,.306,.88)
  b=continuous_section_angles(fs,-.735,-.306,.88)
  for x,y in zip(a,b):self.assertAlmostEqual(x,-y)
  base=(.07,-.06,1.6);p=(.073,-.061,1.602)
  self.assertAlmostEqual(math.dist(base,p),math.dist(base,rotate_section(p,base,.3)))
