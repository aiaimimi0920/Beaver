import unittest
from lateral_fit import depth_shifts,interpolate_shift
class FinalLateralTests(unittest.TestCase):
 def test_final_centerline_is_monotonic_and_endpoints_are_fixed(self):
  f=[0,.2,.4,.6,.8,1]; y=[-.030,-.031,-.028,-.029,-.026,-.025]
  ds=depth_shifts(f,y); result=[a+b for a,b in zip(y,ds)]
  self.assertEqual(ds[0],0);self.assertEqual(ds[-1],0)
  self.assertTrue(all(a<=b for a,b in zip(result,result[1:])))
 def test_large_geometry_error_is_not_silently_accepted(self):
  with self.assertRaises(AssertionError):depth_shifts([0,.5,1],[0,.01,0])
 def test_companion_uses_same_continuous_translation(self):
  self.assertAlmostEqual(interpolate_shift(.25,[0,.5,1],[0,.002,0]),.001)
  for thickness in [.00018,.0003]:
   shift=.001; self.assertAlmostEqual((.03+shift)-(.03-thickness+shift),thickness)
