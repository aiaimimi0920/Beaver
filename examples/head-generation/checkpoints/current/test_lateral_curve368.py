import unittest
from test_spatial_lashes196 import S,STYLE
class LateralCurveTests(unittest.TestCase):
 def test_guide_has_no_secondary_outward_bulge(self):
  p=STYLE['accent_lateral_center_profile']
  values=[S['curve'](i/200,1,p) for i in range(201)]
  self.assertTrue(all(a>=b-1e-12 for a,b in zip(values,values[1:])))
 def test_endpoint_and_root_blend_are_semantically_bounded(self):
  self.assertGreater(STYLE['accent_lateral_bottom_t'],.3)
  self.assertLess(STYLE['accent_lateral_bottom_t'],.8)
  self.assertGreater(STYLE['accent_lateral_join_span'],.5)
  self.assertLess(STYLE['accent_lateral_join_span'],.8)
